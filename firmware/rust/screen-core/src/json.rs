//! JSON lu avec `serde_json` en `no_std + alloc` et écrit comme cJSON.
//!
//! Le visiteur conserve l'ordre et les doublons : le premier gagne pour
//! `get`, le dernier pendant l'application d'un correctif. Le lecteur accepte
//! une première valeur suivie de données, comme `cJSON_Parse`.
//! L'écriture dédiée reste nécessaire : cJSON utilise `%1.15g` / `%1.17g`,
//! et `/config` impose ses décimales ; la sortie standard de serde_json
//! changerait ces octets. Les entrées invalides parfois tolérées par cJSON
//! (contrôles non échappés, nombres mal formés) sont refusées par serde_json.
//! Sa limite de profondeur (128) protège la pile ; cJSON acceptait 1000 niveaux.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Valeur JSON ordonnée.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    /// Texte numérique déjà formaté, inséré tel quel (`cJSON_AddRaw`).
    Raw(String),
    Str(String),
    Array(Vec<Json>),
    /// Paires dans l'ordre du document, doublons compris.
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn object() -> Self {
        Self::Object(Vec::new())
    }

    /// Ajoute une paire à un objet (sans effet sur un autre type).
    pub fn push(&mut self, key: &str, value: Json) {
        if let Self::Object(pairs) = self {
            pairs.push((key.to_string(), value));
        }
    }

    pub fn push_number(&mut self, key: &str, value: f64) {
        self.push(key, Self::Number(value));
    }

    pub fn push_bool(&mut self, key: &str, value: bool) {
        self.push(key, Self::Bool(value));
    }

    pub fn push_str(&mut self, key: &str, value: &str) {
        self.push(key, Self::Str(value.to_string()));
    }

    pub fn push_null(&mut self, key: &str) {
        self.push(key, Self::Null);
    }

    /// Première valeur de clé `key` (`cJSON_GetObjectItemCaseSensitive`).
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Self::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Objet fils (`cJSON_AddObjectToObject`) : renvoie une référence à
    /// compléter.
    pub fn push_object(&mut self, key: &str) -> &mut Json {
        let Self::Object(pairs) = self else {
            unreachable!("push_object sur un non-objet");
        };
        pairs.push((key.to_string(), Json::object()));
        &mut pairs.last_mut().expect("paire ajoutée").1
    }

    /// Accès mutable au premier fils `key`.
    pub fn child_mut(&mut self, key: &str) -> Option<&mut Json> {
        match self {
            Self::Object(pairs) => pairs.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::Str(_))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Sérialisation compacte (`cJSON_PrintUnformatted`).
    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(true) => out.push_str("true"),
            Self::Bool(false) => out.push_str("false"),
            Self::Number(n) => out.push_str(&number_to_string(*n)),
            Self::Raw(r) => out.push_str(r),
            Self::Str(s) => write_string(out, s),
            Self::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Self::Object(pairs) => {
                out.push('{');
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(out, key);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// Chaîne entre guillemets : seuls `"`, `\` et les caractères de contrôle sont
/// échappés, comme `print_string_ptr`.
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 32 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `printf("%.{precision}g", value)` de la libc.
pub fn format_g(value: f64, precision: usize) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    let precision = precision.max(1);
    let negative = value.is_sign_negative();
    let sign = if negative { "-" } else { "" };
    if value == 0.0 {
        return format!("{sign}0");
    }
    let scientific = format!("{:.*e}", precision - 1, value.abs());
    let (mantissa, exp) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    if exp < -4 || exp >= precision as i32 {
        let mut mant = String::new();
        mant.push_str(&digits[..1]);
        let frac = digits[1..].trim_end_matches('0');
        if !frac.is_empty() {
            mant.push('.');
            mant.push_str(frac);
        }
        let exp_sign = if exp < 0 { '-' } else { '+' };
        return format!("{sign}{mant}e{exp_sign}{:02}", exp.abs());
    }
    if exp >= 0 {
        let split = exp as usize + 1;
        let int_part = &digits[..split];
        let frac = digits[split..].trim_end_matches('0');
        if frac.is_empty() {
            format!("{sign}{int_part}")
        } else {
            format!("{sign}{int_part}.{frac}")
        }
    } else {
        let zeros = "0".repeat((-exp - 1) as usize);
        let frac = format!("{zeros}{digits}");
        let frac = frac.trim_end_matches('0');
        format!("{sign}0.{frac}")
    }
}

/// Nombre tel que `print_number` de cJSON l'écrit.
pub fn number_to_string(d: f64) -> String {
    if !d.is_finite() {
        return "null".to_string();
    }
    let value_int: i32 = if d >= f64::from(i32::MAX) {
        i32::MAX
    } else if d <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        d as i32
    };
    if d == f64::from(value_int) {
        return value_int.to_string();
    }
    let short = format_g(d, 15);
    let same = short.parse::<f64>().is_ok_and(|test| {
        let max = libm::fabs(d).max(libm::fabs(test));
        libm::fabs(test - d) <= max * f64::EPSILON
    });
    if same { short } else { format_g(d, 17) }
}

// --- Lecture -------------------------------------------------------------------

impl<'de> serde::Deserialize<'de> for Json {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct OrderedVisitor;
        impl<'de> serde::de::Visitor<'de> for OrderedVisitor {
            type Value = Json;
            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("une valeur JSON")
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Json, E> {
                Ok(Json::Null)
            }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Json, E> {
                Ok(Json::Bool(value))
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Json, E> {
                Ok(Json::Number(value as f64))
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Json, E> {
                Ok(Json::Number(value as f64))
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Json, E> {
                Ok(Json::Number(value))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Json, E> {
                Ok(Json::Str(value.to_string()))
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Json, E> {
                Ok(Json::Str(value))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Json, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element()? {
                    values.push(value);
                }
                Ok(Json::Array(values))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
                let mut pairs = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    pairs.push(pair);
                }
                Ok(Json::Object(pairs))
            }
        }
        deserializer.deserialize_any(OrderedVisitor)
    }
}

/// Lit la première valeur avec serde_json, sans vérifier la suite, comme
/// `cJSON_Parse`. Les doublons sont conservés. `None` si invalide.
pub fn parse(text: &[u8]) -> Option<Json> {
    let text = text.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(text);
    let mut deserializer = serde_json::Deserializer::from_slice(text);
    serde::Deserialize::deserialize(&mut deserializer).ok()
}
