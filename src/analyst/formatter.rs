pub mod duration {
    use chrono::Duration;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let t_ms = duration.num_milliseconds();
        let mut n_s = t_ms / 1000;
        let n_ms = t_ms - 1000 * n_s;
        let n_m = n_s / 60;
        n_s = n_s - 60 * n_m;
        let s = format!("{}m {}s and {}ms", n_m, n_s, n_ms);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }
}

pub mod datetime {
    use chrono::{Duration, NaiveDateTime};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(datetime: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(datetime: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }
}

pub mod number {
    use num_format::{Locale, ToFormattedString};
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = value.to_formatted_string(&Locale::en);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(value: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }
}

pub mod number_options {
    use num_format::{Locale, ToFormattedString};
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<f64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            None => return serializer.serialize_str(""),
            Some(v) => {
                let n = *v as u64;
                let s = n.to_formatted_string(&Locale::en);
                return serializer.serialize_str(&s);
            }
        }
    }

    pub fn deserialize<'de, D>(value: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }
}
