# 🦀 Relatório 1: Auditoria Diagnóstica Inicial — bot-rust

Este documento consolida a auditoria da **1ª Etapa**, focada nas rotinas de **Voz**, **Fechamento Semanal de Metas**, **Handlers de Interação Discord** e **Banco de Dados de Voz**.

---

## 📊 Sumário Executivo de Vulnerabilidades (1ª Etapa)

| Frente de Auditoria | 🔴 Crítica | 🟠 Alta | 🟡 Média | 🟢 Baixa | Total |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **1. Concorrência & Gestão de Estado na RAM** | 0 | 1 | 1 | 0 | **2** |
| **2. Banco de Dados & Integridade SQLx** | 0 | 2 | 1 | 1 | **4** |
| **3. Robustez & Prevenção de Panics / Crashes** | 1 | 2 | 2 | 0 | **5** |
| **4. Performance, Pipeline Gráfico & Crons** | 1 | 0 | 1 | 0 | **2** |
| **TOTAL** | **2** | **5** | **5** | **1** | **13** |

---

## 🧵 1. Frente 1: Concorrência & Gestão de Estado na RAM

### [ALTA] Perda Total de Tempo de Voz em Reinicializações e Deploys
* **Arquivo:** [`src/events/voice.rs:32-68`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/events/voice.rs#L32-L68)
* **Diagnóstico:** O tempo de voz do usuário só era persistido no PostgreSQL no momento em que ele desconectava da sala (`new.channel_id.is_none()`). Se o bot fosse reiniciado (ex: novo deploy no Fly.io ou reinício do container) enquanto 50 pessoas estavam em call há 3 horas, todo o tempo dessa sessão na RAM (`Arc<DashMap>`) era perdido permanentemente.
* **Proposta de Correção:**
  Criar um cron em background que a cada 5 ou 10 minutos faz um flush incremental dos deltas de tempo ativos no banco para todos os usuários atualmente no `tracker`, atualizando o `joined_at` para o timestamp atual.

### [MÉDIA] Risco de `valid_time_ms` Negativo por Drift de Relógio
* **Arquivo:** [`src/events/voice.rs:60`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/events/voice.rs#L60)
* **Diagnóstico:**
  ```rust
  let total_time_ms = chrono::Utc::now().timestamp_millis() - join.joined_at;
  let valid_time_ms = total_time_ms - join.total_muted;
  ```
  Se houver pequenos drifts de relógio NTP entre o momento do mute e a saída, ou desordem de pacotes de Gateway, `join.total_muted` pode ser marginalmente maior que `total_time_ms`, gerando um número negativo no banco.
* **Proposta de Correção:**
  ```rust
  let valid_time_ms = total_time_ms.saturating_sub(join.total_muted);
  ```

---

## 🗄️ 2. Frente 2: Banco de Dados & Integridade SQLx

### [ALTA] Gargalo Crítico N+1 em `get_all_users_closing_stats`
* **Arquivo:** [`src/database/voice.rs:145-150`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/database/voice.rs#L145-L150)
* **Diagnóstico:**
  ```sql
  SELECT u.id_usuario,
         (SELECT SUM(tempo) FROM sessoes_voz s WHERE s.id_usuario = u.id_usuario AND s.data_sessao >= NOW() - INTERVAL '7 days') as week_ms,
         (SELECT CAST(EXTRACT(DAY FROM (NOW() - MAX(data_sessao))) AS INTEGER) FROM sessoes_voz s WHERE s.id_usuario = u.id_usuario) as inactive_days
  FROM usuarios u
  ```
  Para cada usuário na tabela `usuarios`, o Postgres executa duas subqueries correlacionadas em `sessoes_voz`. Em um banco com milhares de usuários e dezenas de milhares de sessões, isso gera um gargalo massivo de I/O ($O(N)$ scans) e risco de timeout no Supabase durante o fechamento.
* **Proposta de Correção:**
  Reescrever com agregação única via `LEFT JOIN`:
  ```sql
  SELECT 
      u.id_usuario,
      COALESCE(SUM(CASE WHEN s.data_sessao >= NOW() - INTERVAL '7 days' THEN s.tempo ELSE 0 END), 0) as week_ms,
      COALESCE(EXTRACT(DAY FROM (NOW() - MAX(s.data_sessao)))::INTEGER, 999) as inactive_days
  FROM usuarios u
  LEFT JOIN sessoes_voz s ON s.id_usuario = u.id_usuario
  GROUP BY u.id_usuario
  ```

### [ALTA] Crash Fatal no Boot por `.expect()` no `BlacklistDb::init`
* **Arquivo:** [`src/database/blacklist.rs:35, 48`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/database/blacklist.rs#L35)
* **Diagnóstico:** Em `BlacklistDb::init`, as queries usavam `.expect("Failed to create blacklist_panels table")`. Se houver instabilidade transitória de DNS ou conexão com o pooler do Supabase no exato instante em que o container sobe, o processo sofre um `panic!` imediato e o container entra em crash loop.
* **Proposta de Correção:** Substituir `.expect(...)` por `if let Err(e) = ... { error!(...); }` como feito no `VoiceDb` e `TicketDb`.

### [MÉDIA] Ausência de Índice em `usuarios(tempo_total)`
* **Arquivo:** [`src/database/voice.rs:104-107`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/database/voice.rs#L104-L107)
* **Diagnóstico:** O comando `/tempo` executa `SELECT COUNT(*) FROM usuarios WHERE tempo_total > $1`. Como a coluna `tempo_total` não possui índice, cada invocação do card de tempo força um Sequential Scan em toda a tabela `usuarios`.
* **Proposta de Correção:**
  Adicionar no `VoiceDb::init`:
  ```sql
  CREATE INDEX IF NOT EXISTS idx_usuarios_tempo_total ON usuarios (tempo_total DESC);
  ```

---

## 🛡️ 3. Frente 3: Robustez, Tratamento de Erros & Prevenção de Panics

### [CRÍTICA] Múltiplos `.unwrap()` em Handlers de Interação Discord
* **Arquivo:** [`src/events/interactions.rs:187, 600, 890, 897, 928`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/events/interactions.rs#L187)
* **Diagnóstico:**
  - `let guild_id = component.guild_id.unwrap();` crasheia se uma interação ocorrer fora de uma guild (ex: mensagem direta ou contexto global).
  - `let member = modal.guild_id.unwrap().member(&ctx.http, uid).await.unwrap();` causa **panic fatal na task** se o usuário tiver saído do servidor antes de submeter o modal (a API do Discord retorna 404 Not Found).
* **Proposta de Correção:**
  Usar *early returns* seguros com `let Some(guild_id) = component.guild_id else { return; }` e casar o `Result` do `member`:
  ```rust
  let Ok(member) = guild_id.member(&ctx.http, uid).await else {
      error!("Membro {} não encontrado na guild", uid);
      return;
  };
  ```

### [ALTA] `.unwrap()` no Parsing de Fontes no Gerador de Cards
* **Arquivo:** [`src/commands/voice/tempo.rs:209, 381`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/commands/voice/tempo.rs#L209)
* **Diagnóstico:**
  `let font = Font::try_from_vec(font_data).unwrap();`
  Se a leitura da fonte falhar no boot ou o arquivo estiver corrompido, `font_data` é um `vec![]` vazio, levando a um `panic!` toda vez que `/tempo` for chamado.
* **Proposta de Correção:** Fazer o parse da fonte uma única vez no boot e armazenar `Arc<Font<'static>>` já validado no `TypeMap`.

### [MÉDIA] `.unwrap()` em `strip_prefix` de Custom IDs
* **Arquivo:** [`src/events/interactions.rs:85, 181, 702, 761, 777, 800`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/events/interactions.rs#L85)
* **Diagnóstico:** Chamar `.strip_prefix(...).unwrap()` diretamente causa panic caso um botão legado ou de versão anterior envie um custom_id inesperado.
* **Proposta de Correção:** Substituir por `let Some(opt_id) = modal.data.custom_id.strip_prefix(...) else { return; };`.

---

## ⚡ 4. Frente 4: Performance, Pipeline Gráfico & Crons

### [CRÍTICA] Cron de Fechamento Semanal (`fechamento.rs`) é um Stub sem Execução
* **Arquivo:** [`src/cron/fechamento.rs:30-34`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/cron/fechamento.rs#L30-L34)
* **Diagnóstico:**
  ```rust
  if days_until_reset == 0 {
      info!("Fechamento Semanal Automático Iniciado (Horário de Brasília)!");
      sleep(Duration::from_secs(60)).await;
  }
  ```
  O cron calcula perfeitamente o fuso de Brasília e o horário de virada de domingo para segunda (00:00). Porém, ao disparar, ele **apenas emite um log e volta a dormir**, sem executar a lógica real de rebaixamento de patentes e inatividade (que ficou isolada no comando manual `/fechar_metas`).
* **Proposta de Correção:**
  Extrair a lógica de fechamento de `src/commands/metas/fechar_metas.rs` para uma função compartilhada de serviço e chamá-la dentro do cron, postando o relatório no canal de logs da staff.

### [MÉDIA] Re-parsing do Vetor TTF a Cada Execução de `/tempo`
* **Arquivo:** [`src/commands/voice/tempo.rs:207-210`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/commands/voice/tempo.rs#L207-L210)
* **Diagnóstico:** Cada comando `/tempo` executa `Font::try_from_vec` sobre centenas de kilobytes de dados de fonte, alocando tabelas de glifos repetidamente na heap.
* **Proposta de Correção:** Manter o `Font<'static>` instanciado em cache estático global (`LazyLock` ou `TypeMap`).
