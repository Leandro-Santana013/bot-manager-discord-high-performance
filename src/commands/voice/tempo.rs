use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::prelude::*;
use std::io::Cursor;
use image::{RgbaImage, Rgba};
use imageproc::drawing::{draw_text_mut, draw_line_segment_mut, draw_hollow_circle_mut};
use rusttype::{Font, Scale};

pub fn register() -> CreateCommand {
    CreateCommand::new("tempo")
        .description("Gera um card super avançado mostrando seu tempo de call")
}

fn hex_to_rgba(hex: &str) -> Rgba<u8> {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    Rgba([r, g, b, 255])
}

fn draw_rounded_rect(image: &mut RgbaImage, x: i32, y: i32, width: i32, height: i32, radius: i32, color: Rgba<u8>, border_color: Option<Rgba<u8>>) {
    let radius = radius as f32;
    let hw = width as f32 / 2.0;
    let hh = height as f32 / 2.0;
    let cx = x as f32 + hw;
    let cy = y as f32 + hh;
    for iy in (y - 1)..(y + height + 1) {
        for ix in (x - 1)..(x + width + 1) {
            if ix < 0 || iy < 0 || ix >= image.width() as i32 || iy >= image.height() as i32 { continue; }
            let px = (ix as f32 + 0.5 - cx).abs() - hw + radius;
            let py = (iy as f32 + 0.5 - cy).abs() - hh + radius;
            let dist = px.max(0.0).hypot(py.max(0.0)) + px.max(py).min(0.0) - radius;
            
            if dist > 1.0 { continue; }
            
            let alpha = (1.0 - dist).clamp(0.0, 1.0);
            let mut target_color = color;
            
            if let Some(bc) = border_color {
                if dist > -1.5 {
                    let border_alpha = (dist + 1.5).clamp(0.0, 1.0);
                    target_color = Rgba([
                        (bc[0] as f32 * border_alpha + color[0] as f32 * (1.0 - border_alpha)) as u8,
                        (bc[1] as f32 * border_alpha + color[1] as f32 * (1.0 - border_alpha)) as u8,
                        (bc[2] as f32 * border_alpha + color[2] as f32 * (1.0 - border_alpha)) as u8,
                        255
                    ]);
                }
            }
            
            if alpha > 0.0 {
                let mut p = *image.get_pixel(ix as u32, iy as u32);
                let a = alpha;
                let inv = 1.0 - a;
                p[0] = (target_color[0] as f32 * a + p[0] as f32 * inv) as u8;
                p[1] = (target_color[1] as f32 * a + p[1] as f32 * inv) as u8;
                p[2] = (target_color[2] as f32 * a + p[2] as f32 * inv) as u8;
                p[3] = (target_color[3] as f32 * a + p[3] as f32 * inv) as u8;
                image.put_pixel(ix as u32, iy as u32, p);
            }
        }
    }
}

fn ms_to_time(ms: i64) -> String {
    if ms <= 0 { return "0h 0m 0s".to_string(); }
    let seconds = (ms / 1000) % 60;
    let minutes = (ms / (1000 * 60)) % 60;
    let hours = ms / (1000 * 60 * 60);
    format!("{}h {}m {}s", hours, minutes, seconds)
}

fn get_week_string(offset_start: i64, offset_end: i64) -> String {
    use chrono::{Duration, Utc, Datelike};
    let start = Utc::now() - Duration::days(offset_start);
    let end = Utc::now() - Duration::days(offset_end);
    format!("({:02}/{:02} - {:02}/{:02})", start.day(), start.month(), end.day(), end.month())
}

fn draw_aa_circle_icon(image: &mut RgbaImage, cx: i32, cy: i32, radius: f32, thickness: f32, color: Rgba<u8>) {
    let r_outer = radius + thickness / 2.0;
    for y in (cy - r_outer as i32 - 2)..=(cy + r_outer as i32 + 2) {
        for x in (cx - r_outer as i32 - 2)..=(cx + r_outer as i32 + 2) {
            if x < 0 || y < 0 || x >= image.width() as i32 || y >= image.height() as i32 { continue; }
            let dx = x as f32 - cx as f32;
            let dy = y as f32 - cy as f32;
            let dist = (dx*dx + dy*dy).sqrt();
            let dist_to_line = (dist - radius).abs();
            if dist_to_line <= thickness / 2.0 {
                let mut p = *image.get_pixel(x as u32, y as u32);
                let a = 1.0 - (dist_to_line - (thickness / 2.0 - 1.0)).clamp(0.0, 1.0);
                p[0] = (color[0] as f32 * a + p[0] as f32 * (1.0 - a)) as u8;
                p[1] = (color[1] as f32 * a + p[1] as f32 * (1.0 - a)) as u8;
                p[2] = (color[2] as f32 * a + p[2] as f32 * (1.0 - a)) as u8;
                p[3] = (color[3] as f32 * a + p[3] as f32 * (1.0 - a)) as u8;
                image.put_pixel(x as u32, y as u32, p);
            }
        }
    }
}

fn draw_aa_line(image: &mut RgbaImage, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Rgba<u8>) {
    let min_x = (x1.min(x2) - thickness).floor() as i32;
    let max_x = (x1.max(x2) + thickness).ceil() as i32;
    let min_y = (y1.min(y2) - thickness).floor() as i32;
    let max_y = (y1.max(y2) + thickness).ceil() as i32;
    let l2 = (x1 - x2).powi(2) + (y1 - y2).powi(2);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if x < 0 || y < 0 || x >= image.width() as i32 || y >= image.height() as i32 { continue; }
            let dist = if l2 == 0.0 {
                ((x as f32 - x1).powi(2) + (y as f32 - y1).powi(2)).sqrt()
            } else {
                let t = (((x as f32 - x1) * (x2 - x1) + (y as f32 - y1) * (y2 - y1)) / l2).clamp(0.0, 1.0);
                let proj_x = x1 + t * (x2 - x1);
                let proj_y = y1 + t * (y2 - y1);
                ((x as f32 - proj_x).powi(2) + (y as f32 - proj_y).powi(2)).sqrt()
            };
            if dist <= thickness / 2.0 + 0.5 {
                let alpha = (thickness / 2.0 + 0.5 - dist).clamp(0.0, 1.0);
                let mut p = *image.get_pixel(x as u32, y as u32);
                let a = alpha;
                let inv = 1.0 - a;
                p[0] = (color[0] as f32 * a + p[0] as f32 * inv) as u8;
                p[1] = (color[1] as f32 * a + p[1] as f32 * inv) as u8;
                p[2] = (color[2] as f32 * a + p[2] as f32 * inv) as u8;
                p[3] = (color[3] as f32 * a + p[3] as f32 * inv) as u8;
                image.put_pixel(x as u32, y as u32, p);
            }
        }
    }
}

fn draw_clock_icon(image: &mut RgbaImage, cx: i32, cy: i32, color: Rgba<u8>) {
    draw_aa_circle_icon(image, cx, cy, 22.0, 2.5, color);
    draw_aa_line(image, cx as f32, (cy - 12) as f32, cx as f32, cy as f32, 2.5, color);
    draw_aa_line(image, cx as f32, cy as f32, (cx + 8) as f32, (cy + 8) as f32, 2.5, color);
}

fn draw_mute_icon(image: &mut RgbaImage, cx: i32, cy: i32, color: Rgba<u8>) {
    draw_aa_circle_icon(image, cx, cy, 22.0, 2.5, color);
    draw_aa_line(image, (cx - 15) as f32, (cy - 15) as f32, (cx + 15) as f32, (cy + 15) as f32, 2.5, color);
    draw_aa_line(image, (cx - 14) as f32, (cy - 15) as f32, (cx + 16) as f32, (cy + 15) as f32, 2.5, color);
    draw_aa_line(image, (cx - 6) as f32, (cy - 4) as f32, (cx - 6) as f32, (cy + 4) as f32, 2.5, color);
    draw_aa_line(image, (cx - 6) as f32, (cy - 4) as f32, (cx + 2) as f32, (cy - 8) as f32, 2.5, color);
    draw_aa_line(image, (cx - 6) as f32, (cy + 4) as f32, (cx + 2) as f32, (cy + 8) as f32, 2.5, color);
    draw_aa_line(image, (cx + 2) as f32, (cy - 8) as f32, (cx + 2) as f32, (cy + 8) as f32, 2.5, color);
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await;

    let user_id_str = interaction.user.id.to_string();

    // 1. Fetch DB Stats & Live Stats
    let mut stats = {
        let data = ctx.data.read().await;
        let pool = data.get::<crate::DatabasePool>().expect("DB").clone();
        crate::database::voice::VoiceDb::get_user_stats(&pool, &user_id_str).await
    };

    {
        let data = ctx.data.read().await;
        if let Some(tracker) = data.get::<crate::events::voice::VoiceTracker>() {
            if let Some(join) = tracker.get(&user_id_str) {
                let now = chrono::Utc::now().timestamp_millis();
                let active_ms = now - join.joined_at;
                let mut active_muted = join.total_muted;
                if let Some(last_mute) = join.last_mute_at {
                    active_muted += now - last_mute;
                }
                let active_valid = active_ms - active_muted;
                
                stats.total_ms += active_valid;
                stats.this_week_ms += active_valid;
                stats.this_week_muted_ms += active_muted;
            }
        }
    }

    let canvas_w = 800;
    let canvas_h = 620;
    let mut image = RgbaImage::new(canvas_w as u32, canvas_h as u32);
    
    draw_rounded_rect(&mut image, 0, 0, canvas_w, canvas_h, 25, hex_to_rgba("#111214"), None);

    let data = ctx.data.read().await;
    let font_data = data.get::<crate::FontCache>().cloned().unwrap_or_else(|| vec![]);
    let font_bold_data = data.get::<crate::FontCacheBold>().cloned().unwrap_or_else(|| vec![]);
    let font = Font::try_from_vec(font_data).unwrap();
    let font_bold = Font::try_from_vec(font_bold_data).unwrap_or(font.clone());

    let avatar_size: i32 = 140;
    let avatar_x: i32 = 40;
    let avatar_y: i32 = 80;

    let avatar_url = interaction.user.avatar_url().unwrap_or_else(|| "https://cdn.discordapp.com/embed/avatars/0.png".to_string());
    
    let req_client = {
        let data = ctx.data.read().await;
        data.get::<crate::HttpClient>().cloned().unwrap_or_else(reqwest::Client::new)
    };

    if let Ok(resp) = req_client.get(&avatar_url).send().await {
        if let Ok(bytes) = resp.bytes().await {
            if let Ok(img) = image::load_from_memory(&bytes) {
                let img = image::imageops::resize(&img.to_rgba8(), avatar_size as u32, avatar_size as u32, image::imageops::FilterType::Lanczos3);
                let mut masked = RgbaImage::new(avatar_size as u32, avatar_size as u32);
                let c = avatar_size as f32 / 2.0;
                for iy in 0..avatar_size {
                    for ix in 0..avatar_size {
                        let dx = ix as f32 - c + 0.5;
                        let dy = iy as f32 - c + 0.5;
                        let dist = (dx*dx + dy*dy).sqrt();
                        if dist <= c {
                            let mut p = *img.get_pixel(ix as u32, iy as u32);
                            if dist > c - 1.5 {
                                let alpha = ((c - dist) / 1.5).clamp(0.0, 1.0);
                                p[3] = (p[3] as f32 * alpha) as u8;
                            }
                            masked.put_pixel(ix as u32, iy as u32, p);
                        }
                    }
                }
                image::imageops::overlay(&mut image, &masked, avatar_x as i64, avatar_y as i64);
            }
        }
    }

    // Textos Superiores
    let username_x = avatar_x + avatar_size + 20;
    let username_y = avatar_y + 15;
    
    draw_text_mut(&mut image, hex_to_rgba("#ffffff"), username_x, username_y, Scale::uniform(36.0), &font_bold, &interaction.user.name);
    draw_text_mut(&mut image, hex_to_rgba("#999999"), username_x, username_y + 45, Scale::uniform(20.0), &font_bold, &format!("Tempo total: {}", ms_to_time(stats.total_ms)));
    
    let rank_text = format!("Rank Atual: #{}", stats.rank);
    let w = font_bold.layout(&rank_text, Scale::uniform(24.0), rusttype::Point { x: 0.0, y: 0.0 }).last().map(|g| g.position().x + g.unpositioned().h_metrics().advance_width).unwrap_or(150.0);
    draw_text_mut(&mut image, hex_to_rgba("#ffcc00"), canvas_w - 40 - w as i32, username_y + 20, Scale::uniform(24.0), &font_bold, &rank_text);

    let start_cols_y = avatar_y + avatar_size + 20;
    draw_line_segment_mut(&mut image, (0.0, start_cols_y as f32), (canvas_w as f32, start_cols_y as f32), hex_to_rgba("#222222"));
    draw_line_segment_mut(&mut image, (canvas_w as f32 / 2.0, start_cols_y as f32), (canvas_w as f32 / 2.0, (canvas_h - 80) as f32), hex_to_rgba("#222222"));

    let col_w = canvas_w / 2;
    let col1_x = 0;
    let col2_x = col_w;

    let week1_str = get_week_string(7, 0);
    let week2_str = get_week_string(14, 7);

    let draw_centered = |img: &mut RgbaImage, x_center: i32, y: i32, scale: Scale, font: &Font, text: &str, color: Rgba<u8>| {
        let w = font.layout(text, scale, rusttype::Point { x: 0.0, y: 0.0 }).last().map(|g| g.position().x + g.unpositioned().h_metrics().advance_width).unwrap_or(0.0) as i32;
        draw_text_mut(img, color, x_center - (w / 2), y, scale, font, text);
    };

    // Titulos
    draw_centered(&mut image, col1_x + (col_w/2), start_cols_y + 25, Scale::uniform(22.0), &font_bold, "TEMPO DA SEMANA", hex_to_rgba("#ffffff"));
    draw_centered(&mut image, col1_x + (col_w/2), start_cols_y + 55, Scale::uniform(16.0), &font, &week1_str, hex_to_rgba("#777777"));

    draw_centered(&mut image, col2_x + (col_w/2), start_cols_y + 25, Scale::uniform(22.0), &font_bold, "TEMPO DA SEMANA PASSADA", hex_to_rgba("#ffffff"));
    draw_centered(&mut image, col2_x + (col_w/2), start_cols_y + 55, Scale::uniform(16.0), &font, &week2_str, hex_to_rgba("#777777"));

    let box_w = col_w - 40;
    let box_h = 90;
    let box_y1 = start_cols_y + 90;
    let box_y2 = start_cols_y + 195;

    // BOX 1 (Semana Atual)
    draw_rounded_rect(&mut image, col1_x + 20, box_y1, box_w, box_h, 20, hex_to_rgba("#111111"), Some(hex_to_rgba("#444444")));
    draw_clock_icon(&mut image, col1_x + 60, box_y1 + box_h/2, hex_to_rgba("#cccccc"));
    draw_text_mut(&mut image, hex_to_rgba("#ffffff"), col1_x + 103, box_y1 + 20, Scale::uniform(26.0), &font_bold, &ms_to_time(stats.this_week_ms));
    draw_text_mut(&mut image, hex_to_rgba("#888888"), col1_x + 105, box_y1 + 52, Scale::uniform(16.0), &font, "Tempo total da semana.");

    // BOX 2 (Mutado Semana Atual)
    draw_rounded_rect(&mut image, col1_x + 20, box_y2, box_w, box_h, 20, hex_to_rgba("#1a0505"), Some(hex_to_rgba("#551111")));
    draw_mute_icon(&mut image, col1_x + 60, box_y2 + box_h/2, hex_to_rgba("#cc4444"));
    draw_text_mut(&mut image, hex_to_rgba("#ffffff"), col1_x + 103, box_y2 + 20, Scale::uniform(26.0), &font_bold, &ms_to_time(stats.this_week_muted_ms));
    draw_text_mut(&mut image, hex_to_rgba("#888888"), col1_x + 105, box_y2 + 52, Scale::uniform(16.0), &font, "Tempo que passou mutado.");

    // BOX 3 (Semana Passada)
    draw_rounded_rect(&mut image, col2_x + 20, box_y1, box_w, box_h, 20, hex_to_rgba("#111111"), Some(hex_to_rgba("#444444")));
    draw_clock_icon(&mut image, col2_x + 60, box_y1 + box_h/2, hex_to_rgba("#cccccc"));
    draw_text_mut(&mut image, hex_to_rgba("#ffffff"), col2_x + 103, box_y1 + 20, Scale::uniform(26.0), &font_bold, &ms_to_time(stats.last_week_ms));
    draw_text_mut(&mut image, hex_to_rgba("#888888"), col2_x + 105, box_y1 + 52, Scale::uniform(16.0), &font, "Tempo total da semana.");

    // BOX 4 (Mutado Semana Passada)
    draw_rounded_rect(&mut image, col2_x + 20, box_y2, box_w, box_h, 20, hex_to_rgba("#1a0505"), Some(hex_to_rgba("#551111")));
    draw_mute_icon(&mut image, col2_x + 60, box_y2 + box_h/2, hex_to_rgba("#cc4444"));
    draw_text_mut(&mut image, hex_to_rgba("#ffffff"), col2_x + 103, box_y2 + 20, Scale::uniform(26.0), &font_bold, &ms_to_time(stats.last_week_muted_ms));
    draw_text_mut(&mut image, hex_to_rgba("#888888"), col2_x + 105, box_y2 + 52, Scale::uniform(16.0), &font, "Tempo que passou mutado.");

    // Banner Rodapé (Meta)
    let banner_w = 600;
    let banner_h = 40;
    let banner_x = (canvas_w - banner_w) / 2;
    let banner_y = canvas_h - 60;
    
    draw_rounded_rect(&mut image, banner_x, banner_y, banner_w, banner_h, 20, hex_to_rgba("#330000"), Some(hex_to_rgba("#880000")));
    
    // Ícone de alvo à esquerda
    let icon_x = banner_x + 25;
    let icon_y = banner_y + banner_h / 2;
    draw_hollow_circle_mut(&mut image, (icon_x, icon_y), 8, hex_to_rgba("#ff4444"));
    draw_hollow_circle_mut(&mut image, (icon_x, icon_y), 7, hex_to_rgba("#ff4444"));
    draw_hollow_circle_mut(&mut image, (icon_x, icon_y), 3, hex_to_rgba("#ff4444"));
    draw_hollow_circle_mut(&mut image, (icon_x, icon_y), 2, hex_to_rgba("#ff4444"));
    draw_hollow_circle_mut(&mut image, (icon_x, icon_y), 1, hex_to_rgba("#ff4444"));

    let banner_text = "Tempo para se manter no cargo: Meta do cargo alcançada!";
    draw_centered(&mut image, canvas_w / 2, banner_y + 10, Scale::uniform(18.0), &font_bold, banner_text, hex_to_rgba("#ff4444"));

    let mut buffer = Cursor::new(Vec::new());
    let encoder = image::codecs::png::PngEncoder::new_with_quality(
        &mut buffer,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::NoFilter
    );

    if image::ImageEncoder::write_image(encoder, image.as_raw(), canvas_w as u32, canvas_h as u32, image::ColorType::Rgba8).is_ok() {
        let attachment = serenity::builder::CreateAttachment::bytes(buffer.into_inner(), "perfil-tempo.png");
        let _ = interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new()
            .new_attachment(attachment)
        ).await;
    } else {
        let _ = interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new()
            .content("Erro ao gerar a imagem.")
        ).await;
    }
}

pub async fn run_message(ctx: &Context, msg: &serenity::model::channel::Message) {
    let user_id_str = msg.author.id.to_string();

    let mut stats = {
        let data = ctx.data.read().await;
        let pool = data.get::<crate::DatabasePool>().expect("DB").clone();
        crate::database::voice::VoiceDb::get_user_stats(&pool, &user_id_str).await
    };

    {
        let data = ctx.data.read().await;
        if let Some(tracker) = data.get::<crate::events::voice::VoiceTracker>() {
            if let Some(join) = tracker.get(&user_id_str) {
                let now = chrono::Utc::now().timestamp_millis();
                let active_ms = now - join.joined_at;
                let mut active_muted = join.total_muted;
                if let Some(last_mute) = join.last_mute_at {
                    active_muted += now - last_mute;
                }
                let active_valid = active_ms - active_muted;
                
                stats.total_ms += active_valid;
                stats.this_week_ms += active_valid;
                stats.this_week_muted_ms += active_muted;
            }
        }
    }

    let canvas_w = 800;
    let canvas_h = 620;
    let mut image = RgbaImage::new(canvas_w as u32, canvas_h as u32);
    
    draw_rounded_rect(&mut image, 0, 0, canvas_w, canvas_h, 25, hex_to_rgba("#111214"), None);

    let data = ctx.data.read().await;
    let font_data = data.get::<crate::FontCache>().cloned().unwrap_or_else(|| vec![]);
    let font_bold_data = data.get::<crate::FontCacheBold>().cloned().unwrap_or_else(|| vec![]);
    let font = Font::try_from_vec(font_data).unwrap();
    let font_bold = Font::try_from_vec(font_bold_data).unwrap_or(font.clone());

    let avatar_size: i32 = 140;
    let avatar_x: i32 = 40;
    let avatar_y: i32 = 80;

    let avatar_url = msg.author.avatar_url().unwrap_or_else(|| "https://cdn.discordapp.com/embed/avatars/0.png".to_string());
    if let Ok(resp) = reqwest::get(&avatar_url).await {
        if let Ok(bytes) = resp.bytes().await {
            if let Ok(img) = image::load_from_memory(&bytes) {
                let img = image::imageops::resize(&img.to_rgba8(), avatar_size as u32, avatar_size as u32, image::imageops::FilterType::Lanczos3);
                let mut masked = RgbaImage::new(avatar_size as u32, avatar_size as u32);
                let r2 = (avatar_size as f32 / 2.0).powi(2);
                let c = avatar_size as f32 / 2.0;
                for iy in 0..avatar_size {
                    for ix in 0..avatar_size {
                        let dx = ix as f32 - c + 0.5;
                        let dy = iy as f32 - c + 0.5;
                        if dx*dx + dy*dy <= r2 {
                            masked.put_pixel(ix as u32, iy as u32, *img.get_pixel(ix as u32, iy as u32));
                        }
                    }
                }
                image::imageops::overlay(&mut image, &masked, avatar_x as i64, avatar_y as i64);
            }
        }
    }

    let username_x = avatar_x + avatar_size + 20;
    let username_y = avatar_y + 15;
    
    draw_text_mut(&mut image, hex_to_rgba("#ffffff"), username_x, username_y, Scale::uniform(36.0), &font_bold, &msg.author.name);
    draw_text_mut(&mut image, hex_to_rgba("#999999"), username_x, username_y + 45, Scale::uniform(20.0), &font_bold, &format!("Tempo total: {}", ms_to_time(stats.total_ms)));
    
    let rank_text = format!("Rank Atual: #{}", stats.rank);
    let w = font_bold.layout(&rank_text, Scale::uniform(24.0), rusttype::Point { x: 0.0, y: 0.0 }).last().map(|g| g.position().x + g.unpositioned().h_metrics().advance_width).unwrap_or(150.0);
    draw_text_mut(&mut image, hex_to_rgba("#ffcc00"), canvas_w - 40 - w as i32, username_y + 20, Scale::uniform(24.0), &font_bold, &rank_text);

    let start_cols_y = avatar_y + avatar_size + 20;
    draw_line_segment_mut(&mut image, (0.0, start_cols_y as f32), (canvas_w as f32, start_cols_y as f32), hex_to_rgba("#222222"));
    draw_line_segment_mut(&mut image, (canvas_w as f32 / 2.0, start_cols_y as f32), (canvas_w as f32 / 2.0, (canvas_h - 80) as f32), hex_to_rgba("#222222"));

    let col_w = canvas_w / 2;
    let col1_x = 0;
    let col2_x = col_w;

    let week1_str = get_week_string(7, 0);
    let week2_str = get_week_string(14, 7);

    let draw_centered = |img: &mut RgbaImage, x_center: i32, y: i32, scale: Scale, font: &Font, text: &str, color: Rgba<u8>| {
        let w = font.layout(text, scale, rusttype::Point { x: 0.0, y: 0.0 }).last().map(|g| g.position().x + g.unpositioned().h_metrics().advance_width).unwrap_or(0.0) as i32;
        draw_text_mut(img, color, x_center - (w / 2), y, scale, font, text);
    };

    draw_centered(&mut image, col1_x + (col_w/2), start_cols_y + 25, Scale::uniform(22.0), &font_bold, "TEMPO DA SEMANA", hex_to_rgba("#ffffff"));
    draw_centered(&mut image, col1_x + (col_w/2), start_cols_y + 55, Scale::uniform(16.0), &font_bold, &week1_str, hex_to_rgba("#777777"));
    draw_centered(&mut image, col2_x + (col_w/2), start_cols_y + 25, Scale::uniform(22.0), &font_bold, "TEMPO DA SEMANA PASSADA", hex_to_rgba("#ffffff"));
    draw_centered(&mut image, col2_x + (col_w/2), start_cols_y + 55, Scale::uniform(16.0), &font_bold, &week2_str, hex_to_rgba("#777777"));

    let box_w = col_w - 40;
    let box_h = 90;
    let box_y1 = start_cols_y + 90;
    let box_y2 = start_cols_y + 195;

    draw_rounded_rect(&mut image, col1_x + 20, box_y1, box_w, box_h, 20, hex_to_rgba("#111111"), Some(hex_to_rgba("#444444")));
    draw_clock_icon(&mut image, col1_x + 60, box_y1 + box_h/2, hex_to_rgba("#cccccc"));
    draw_text_mut(&mut image, hex_to_rgba("#ffffff"), col1_x + 105, box_y1 + 35, Scale::uniform(22.0), &font_bold, &ms_to_time(stats.this_week_ms));
    draw_text_mut(&mut image, hex_to_rgba("#888888"), col1_x + 105, box_y1 + 60, Scale::uniform(14.0), &font, "Tempo total da semana.");

    draw_rounded_rect(&mut image, col1_x + 20, box_y2, box_w, box_h, 20, hex_to_rgba("#1a0505"), Some(hex_to_rgba("#551111")));
    draw_mute_icon(&mut image, col1_x + 60, box_y2 + box_h/2, hex_to_rgba("#cc4444"));
    draw_text_mut(&mut image, hex_to_rgba("#ffffff"), col1_x + 105, box_y2 + 35, Scale::uniform(22.0), &font_bold, &ms_to_time(stats.this_week_muted_ms));
    draw_text_mut(&mut image, hex_to_rgba("#888888"), col1_x + 105, box_y2 + 60, Scale::uniform(14.0), &font, "Tempo que passou mutado.");

    draw_rounded_rect(&mut image, col2_x + 20, box_y1, box_w, box_h, 20, hex_to_rgba("#111111"), Some(hex_to_rgba("#444444")));
    draw_clock_icon(&mut image, col2_x + 60, box_y1 + box_h/2, hex_to_rgba("#cccccc"));
    draw_text_mut(&mut image, hex_to_rgba("#ffffff"), col2_x + 105, box_y1 + 35, Scale::uniform(22.0), &font_bold, &ms_to_time(stats.last_week_ms));
    draw_text_mut(&mut image, hex_to_rgba("#888888"), col2_x + 105, box_y1 + 60, Scale::uniform(14.0), &font, "Tempo total da semana.");

    draw_rounded_rect(&mut image, col2_x + 20, box_y2, box_w, box_h, 20, hex_to_rgba("#1a0505"), Some(hex_to_rgba("#551111")));
    draw_mute_icon(&mut image, col2_x + 60, box_y2 + box_h/2, hex_to_rgba("#cc4444"));
    draw_text_mut(&mut image, hex_to_rgba("#ffffff"), col2_x + 105, box_y2 + 35, Scale::uniform(22.0), &font_bold, &ms_to_time(stats.last_week_muted_ms));
    draw_text_mut(&mut image, hex_to_rgba("#888888"), col2_x + 105, box_y2 + 60, Scale::uniform(14.0), &font, "Tempo que passou mutado.");

    let banner_w = 600;
    let banner_h = 40;
    let banner_x = (canvas_w - banner_w) / 2;
    let banner_y = canvas_h - 60;
    
    draw_rounded_rect(&mut image, banner_x, banner_y, banner_w, banner_h, 20, hex_to_rgba("#330000"), Some(hex_to_rgba("#880000")));
    
    // Ícone de alvo à esquerda
    let icon_x = banner_x + 25;
    let icon_y = banner_y + banner_h / 2;
    draw_hollow_circle_mut(&mut image, (icon_x, icon_y), 8, hex_to_rgba("#ff4444"));
    draw_hollow_circle_mut(&mut image, (icon_x, icon_y), 7, hex_to_rgba("#ff4444"));
    draw_hollow_circle_mut(&mut image, (icon_x, icon_y), 3, hex_to_rgba("#ff4444"));
    draw_hollow_circle_mut(&mut image, (icon_x, icon_y), 2, hex_to_rgba("#ff4444"));
    draw_hollow_circle_mut(&mut image, (icon_x, icon_y), 1, hex_to_rgba("#ff4444"));

    let banner_text = "Tempo para se manter no cargo: Meta do cargo alcançada!";
    draw_text_mut(&mut image, hex_to_rgba("#ff4444"), canvas_w / 2 - 220, banner_y + 10, Scale::uniform(18.0), &font_bold, banner_text);

    let mut buffer = Cursor::new(Vec::new());
    if image.write_to(&mut buffer, image::ImageFormat::Png).is_ok() {
        let attachment = serenity::builder::CreateAttachment::bytes(buffer.into_inner(), "perfil-tempo.png");
        if let Ok(reply) = msg.channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().add_file(attachment)).await {
            let http = ctx.http.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                let _ = reply.delete(&http).await;
            });
        }
    }
}
