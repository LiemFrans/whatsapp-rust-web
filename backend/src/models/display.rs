pub fn normalize_phone_number(phone: Option<&str>) -> Option<String> {
    let trimmed = phone?.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = if trimmed.starts_with('+') {
        trimmed.to_string()
    } else {
        format!("+{}", trimmed)
    };

    let digits = normalized.trim_start_matches('+');
    if digits.chars().all(|c| c.is_ascii_digit()) && (7..=15).contains(&digits.len()) {
        Some(normalized)
    } else {
        None
    }
}

pub fn is_unusable_display_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return true;
    }

    if trimmed.contains('∙') {
        return true;
    }

    if trimmed.contains("@lid") || trimmed.contains("@g.us") || trimmed.contains("@s.whatsapp.net") {
        return true;
    }

    if trimmed.chars().all(|c| c.is_ascii_digit()) && trimmed.len() > 15 {
        return true;
    }

    false
}

pub fn preferred_display_name(name: Option<&str>, phone: Option<&str>) -> Option<String> {
    if let Some(name) = name {
        let trimmed = name.trim();
        if !is_unusable_display_name(trimmed) {
            return Some(trimmed.to_string());
        }
    }

    normalize_phone_number(phone)
}