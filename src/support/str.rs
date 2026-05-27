use crate::uuid::Uuid;
use rand::distr::{Alphanumeric, SampleString};

pub struct Stringable(String);

impl Stringable {
    pub fn new(val: String) -> Self {
        Self(val)
    }

    /// Convert the string to uppercase
    pub fn upper(mut self) -> Self {
        self.0 = self.0.to_uppercase();
        self
    }

    /// Convert the string to lowercase
    pub fn lower(mut self) -> Self {
        self.0 = self.0.to_lowercase();
        self
    }

    /// Append a string slice to the end of the string
    pub fn append(mut self, val: &str) -> Self {
        self.0.push_str(val);
        self
    }

    /// Prepend a string slice to the beginning of the string
    pub fn prepend(mut self, val: &str) -> Self {
        self.0.insert_str(0, val);
        self
    }

    /// Convert the string to a URL-friendly slug
    pub fn slug(mut self) -> Self {
        self.0 = Str::slug(&self.0);
        self
    }

    /// Get the portion of the string after the first occurrence of a given value
    pub fn after(mut self, search: &str) -> Self {
        self.0 = Str::after(&self.0, search);
        self
    }

    /// Get the portion of the string before the first occurrence of a given value
    pub fn before(mut self, search: &str) -> Self {
        self.0 = Str::before(&self.0, search);
        self
    }

    /// Get the portion of the string between two given values
    pub fn between(mut self, from: &str, to: &str) -> Self {
        self.0 = Str::between(&self.0, from, to);
        self
    }

    /// Limit the number of characters in the string
    pub fn limit(mut self, max: usize, end: &str) -> Self {
        self.0 = Str::limit(&self.0, max, end);
        self
    }

    /// Replace occurrences of a search string with a replacement string
    pub fn replace(mut self, search: &str, replace: &str) -> Self {
        self.0 = Str::replace(search, replace, &self.0);
        self
    }

    /// Retrieve the final String result
    pub fn get(self) -> String {
        self.0
    }
}

pub struct Str;

impl Str {
    /// Membuat instance baru stringable yang lancar (Str::of($value))
    pub fn of(value: &str) -> Stringable {
        Stringable::new(value.to_string())
    }

    /// Menghasilkan UUID (versi 4) sebagai String (Str::uuid())
    pub fn uuid() -> String {
        Uuid::new_v4().to_string()
    }

    /// Menghasilkan string acak alfabet/angka dengan panjang tertentu (Str::random())
    pub fn random(length: usize) -> String {
        Alphanumeric.sample_string(&mut rand::rng(), length)
    }

    /// Mengonversi string menjadi slug ramah URL (Str::slug())
    pub fn slug(title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Get the portion of a string after the first occurrence of a given value
    pub fn after(subject: &str, search: &str) -> String {
        subject.split_once(search).map(|(_, right)| right).unwrap_or(subject).to_string()
    }

    /// Get the portion of a string before the first occurrence of a given value
    pub fn before(subject: &str, search: &str) -> String {
        subject.split_once(search).map(|(left, _)| left).unwrap_or(subject).to_string()
    }

    /// Get the portion of a string between two given values
    pub fn between(subject: &str, from: &str, to: &str) -> String {
        subject.split(from).nth(1).and_then(|s| s.split(to).next()).unwrap_or("").to_string()
    }

    /// Determine if a string contains another substring
    pub fn contains(haystack: &str, needle: &str) -> bool {
        haystack.contains(needle)
    }

    /// Determine if a string starts with a given substring
    pub fn starts_with(subject: &str, needle: &str) -> bool {
        subject.starts_with(needle)
    }

    /// Determine if a string ends with a given substring
    pub fn ends_with(subject: &str, needle: &str) -> bool {
        subject.ends_with(needle)
    }

    /// Determine if a string is a valid UUID
    pub fn is_uuid(value: &str) -> bool {
        Uuid::parse_str(value).is_ok()
    }

    /// Convert a string to lowercase
    pub fn lower(value: &str) -> String {
        value.to_lowercase()
    }

    /// Convert a string to uppercase
    pub fn upper(value: &str) -> String {
        value.to_uppercase()
    }

    /// Limit the number of characters in a string
    pub fn limit(value: &str, max: usize, end: &str) -> String {
        if value.chars().count() <= max {
            value.to_string()
        } else {
            format!("{}{}", value.chars().take(max).collect::<String>(), end)
        }
    }

    /// Replace all occurrences of a search string with a replacement string
    pub fn replace(search: &str, replace: &str, subject: &str) -> String {
        subject.replace(search, replace)
    }
}
