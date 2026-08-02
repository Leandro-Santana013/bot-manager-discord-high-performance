# 🦀 Relatório 2: Auditoria Diagnóstica Expandida — bot-rust

Este documento consolida a auditoria da **2ª Etapa**, focada nas rotinas de **Mercado Pago & VIP**, **Sistema de Tickets**, **Filtros de Automod** e **Comandos Administrativos de Moderação**.

---

## 📊 Sumário Executivo de Vulnerabilidades (2ª Etapa)

| Frente de Auditoria | 🔴 Crítica | 🟠 Alta | 🟡 Média | 🟢 Baixa | Total |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **1. Concorrência & Gestão de Memória** | 0 | 1 | 1 | 0 | **2** |
| **2. Banco de Dados & Gateway Mercado Pago** | 0 | 2 | 0 | 0 | **2** |
| **3. Robustez & Prevenção de Panics em Moderação** | 1 | 1 | 0 | 0 | **2** |
| **4. Performance & Integrações HTTP** | 0 | 2 | 0 | 0 | **2** |
| **TOTAL** | **1** | **6** | **1** | **0** | **8** |

---

## 🧵 1. Frente 1: Concorrência, Memória & Gestão de Tasks

### [ALTA] Leaks de Tasks em Collectors de Paginação
* **Arquivos:** [`src/commands/voice/top.rs:97-124`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/commands/voice/top.rs#L97-L124), [`src/commands/tickets/ranking.rs:97-124`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/commands/tickets/ranking.rs#L97-L124)
* **Diagnóstico:**
  ```rust
  let mut collector = ComponentInteractionCollector::new(&ctx.shard)
      .message_id(msg.id)
      .timeout(Duration::from_secs(60))
      .stream();
  ```
  O collector de botões mantém a stream de eventos do Discord ativa por 60 segundos inteiros mesmo que o usuário saia do canal ou não interaja mais, retendo handles e memória.
* **Proposta de Correção:**
  Implementar navegação stateless com codificação de página no `custom_id` (ex: `rank_page_2`) tratada diretamente no `interactions.rs` sem manter collectors longos na memória.

### [MÉDIA] Contenção de Lock e Alocações no Automod
* **Arquivo:** [`src/events/message.rs:10-14`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/events/message.rs#L10-L14)
* **Diagnóstico:**
  ```rust
  let cache = {
      let data = ctx.data.read().await;
      data.get::<crate::AutomodCache>().expect("AutomodCache not initialized").clone()
  };
  let words = cache.read().await.clone();
  ```
  A cada mensagem enviada no servidor (mesmo de texto normal), o bot adquire o lock do cache e faz uma clonagem completa (`.clone()`) do vetor de palavras bloqueadas na heap.
* **Proposta de Correção:**
  Fazer a iteração sobre o guard de leitura referenciada direta (`let words = cache.read().await;`) sem alocar novo vetor na memória.

---

## 🗄️ 2. Frente 2: Banco de Dados & Gateway de Pagamentos

### [ALTA] Polling de Pagamentos sem Limitação de Janela no Mercado Pago
* **Arquivo:** [`src/cron/payments.rs:25-39`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/cron/payments.rs#L25-L39)
* **Diagnóstico:**
  ```rust
  let pending = PaymentDb::get_pending_payments(&pool).await;
  for (payment_id, user_id_str, package_id) in pending {
      match mp_client.get_payment_status(&payment_id).await { ... }
  }
  ```
  O cron busca todos os pagamentos pendentes sem filtro de data (`created_at`) nem `LIMIT`. Conforme transações são abandonadas por usuários, o loop executará dezenas de requisições HTTP redundantes para o Mercado Pago a cada 60s.
* **Proposta de Correção:**
  Filtrar apenas pagamentos criados nas últimas 24 horas e rodar uma rotina diária de expiração para limpar registros obsoletos.

### [ALTA] Ordem Insegura na Entrega de VIP
* **Arquivo:** [`src/cron/payments.rs:45-56`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/cron/payments.rs#L45-L56)
* **Diagnóstico:**
  ```rust
  let _ = PaymentDb::remove_payment(&pool, &payment_id).await;
  // ...
  let _ = member.add_role(&http, role_id).await;
  ```
  O registro de pagamento é apagado do banco de dados antes da entrega do cargo no Discord. Se a API do Discord retornar erro 500 ou permissão insuficiente, o pagamento é perdido e o usuário não recebe o produto.
* **Proposta de Correção:**
  Inverter a ordem de execução: registrar o cargo com sucesso primeiro e somente então remover a pendência do banco.

---

## 🛡️ 3. Frente 3: Robustez & Prevenção de Panics em Moderação

### [CRÍTICA] Panic em `edit_response.unwrap()` nos Rankings
* **Arquivos:** [`src/commands/tickets/ranking.rs:94`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/commands/tickets/ranking.rs#L94), [`src/commands/voice/top.rs:137`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/commands/voice/top.rs#L137)
* **Diagnóstico:**
  ```rust
  let mut msg = interaction.edit_response(&ctx.http, response).await.unwrap();
  ```
  Se o token do webhook do Discord expirar (após 15 minutos) ou houver falha transitória de rede, o `.unwrap()` causa panic fatal na task do comando.
* **Proposta de Correção:**
  ```rust
  let Ok(mut msg) = interaction.edit_response(&ctx.http, response).await else {
      return;
  };
  ```

### [ALTA] `.unwrap()` no `guild_id` em Comandos Administrativos
* **Arquivos:** [`src/commands/mod_cmds/blacklist.rs:58`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/commands/mod_cmds/blacklist.rs#L58), [`src/commands/mod_cmds/restringir.rs:52`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/commands/mod_cmds/restringir.rs#L52)
* **Diagnóstico:**
  Desempacotamento de `interaction.guild_id.unwrap()` sem validação derruba a execução se o comando for chamado via contexto global.
* **Proposta de Correção:**
  Usar `let Some(guild_id) = interaction.guild_id else { return; };`.

---

## ⚡ 4. Frente 4: Performance & Integrações HTTP

### [ALTA] Falta de Timeout Explícito no `reqwest::Client` do Mercado Pago
* **Arquivo:** [`src/cron/mercado_pago.rs:57`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/cron/mercado_pago.rs#L57)
* **Diagnóstico:**
  `Client::new()` utiliza o timeout padrão do SO. Se a API do Mercado Pago sofrer degradação ou retenção de conexão TCP, a task do cron de pagamentos pode ficar congelada por minutos.
* **Proposta de Correção:**
  ```rust
  Client::builder()
      .timeout(std::time::Duration::from_secs(10))
      .build()
      .unwrap_or_default()
  ```

### [ALTA] Dependência Exclusiva de Polling para PIX
* **Arquivo:** [`src/cron/payments.rs:14`](file:///c:/Users/Santana/Documents/GitHub/bot-rust/src/cron/payments.rs#L14)
* **Diagnóstico:**
  O sistema depende exclusivamente de um loop a cada 60s. O cliente que paga o PIX aguarda até 1 minuto para a liberação do cargo no servidor.
* **Proposta de Correção:**
  Manter o cron como fallback de segurança e expor uma rota de Webhook HTTP leve para ativação instantânea em sub-segundo.
