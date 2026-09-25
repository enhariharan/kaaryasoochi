// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) 2026, Hariharan Narayanan

//! Static-text translations. One table row per key; columns are
//! `[English, Tamil, Malayalam, Telugu, Hindi]` in `Language::ALL` order.
//! A test guarantees no cell is empty, so adding a key forces all translations.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    En,
    Ta,
    Ml,
    Te,
    Hi,
}

impl Language {
    pub const ALL: [Language; 5] = [Self::En, Self::Ta, Self::Ml, Self::Te, Self::Hi];
    pub fn code(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ta => "ta",
            Self::Ml => "ml",
            Self::Te => "te",
            Self::Hi => "hi",
        }
    }
    /// Name in its own script, for the language picker.
    pub fn native_name(&self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Ta => "தமிழ்",
            Self::Ml => "മലയാളം",
            Self::Te => "తెలుగు",
            Self::Hi => "हिन्दी",
        }
    }
    pub fn parse(code: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|l| l.code() == code)
            .unwrap_or_default()
    }
    fn idx(&self) -> usize {
        Self::ALL.iter().position(|l| l == self).unwrap()
    }
}

/// Look up `key`; falls back to English, then to the key itself.
pub fn tr(lang: Language, key: &str) -> &'static str {
    TABLE
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v[lang.idx()])
        .unwrap_or_else(|| {
            TABLE
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v[0])
                .unwrap_or("")
        })
}

macro_rules! t {
    ($k:literal, $en:literal, $ta:literal, $ml:literal, $te:literal, $hi:literal) => {
        ($k, [$en, $ta, $ml, $te, $hi])
    };
}

pub const TABLE: &[(&str, [&str; 5])] = &[
    t!(
        "app.name",
        "Kaaryasoochi",
        "காரியசூசி",
        "കാര്യസൂചി",
        "కార్యసూచి",
        "कार्यसूची"
    ),
    t!("auth.login", "Log in", "உள்நுழை", "ലോഗിൻ", "లాగిన్", "लॉग इन"),
    t!(
        "auth.logout",
        "Log out",
        "வெளியேறு",
        "ലോഗൗട്ട്",
        "లాగ్ అవుట్",
        "लॉग आउट"
    ),
    t!(
        "auth.register",
        "Create account",
        "கணக்கை உருவாக்கு",
        "അക്കൗണ്ട് സൃഷ്ടിക്കുക",
        "ఖాతా సృష్టించు",
        "खाता बनाएं"
    ),
    t!(
        "auth.username",
        "Username",
        "பயனர்பெயர்",
        "ഉപയോക്തൃനാമം",
        "వినియోగదారు పేరు",
        "उपयोगकर्ता नाम"
    ),
    t!(
        "auth.password",
        "Password",
        "கடவுச்சொல்",
        "പാസ്‌വേഡ്",
        "పాస్‌వర్డ్",
        "पासवर्ड"
    ),
    t!(
        "auth.passkey",
        "Sign in with a passkey",
        "கடவுச்சாவியுடன் உள்நுழை",
        "പാസ്‌കീ ഉപയോഗിച്ച് ലോഗിൻ ചെയ്യുക",
        "పాస్‌కీతో సైన్ ఇన్ చేయండి",
        "पासकी से साइन इन करें"
    ),
    t!(
        "home.title",
        "Dashboard",
        "முகப்பு",
        "ഡാഷ്ബോർഡ്",
        "డాష్‌బోర్డ్",
        "डैशबोर्ड"
    ),
    t!(
        "home.empty",
        "No jobs in this category yet.",
        "இந்த வகையில் பணிகள் இல்லை.",
        "ഈ വിഭാഗത്തിൽ ജോലികളൊന്നുമില്ല.",
        "ఈ వర్గంలో ఇంకా ఉద్యోగాలు లేవు.",
        "इस श्रेणी में अभी कोई कार्य नहीं है।"
    ),
    t!(
        "job.add",
        "Add job",
        "பணியைச் சேர்",
        "ജോലി ചേർക്കുക",
        "ఉద్యోగం జోడించు",
        "कार्य जोड़ें"
    ),
    t!("job.title", "Title", "தலைப்பு", "ശീർഷകം", "శీర్షిక", "शीर्षक"),
    t!("job.category", "Category", "வகை", "വിഭാഗം", "వర్గం", "श्रेणी"),
    t!(
        "job.summary",
        "Summary",
        "சுருக்கம்",
        "സംഗ്രഹം",
        "సారాంశం",
        "सारांश"
    ),
    t!(
        "job.description",
        "Description",
        "விளக்கம்",
        "വിവരണം",
        "వివరణ",
        "विवरण"
    ),
    t!(
        "job.first_run",
        "First run",
        "முதல் இயக்கம்",
        "ആദ്യ റൺ",
        "మొదటి రన్",
        "पहला रन"
    ),
    t!(
        "job.repeat",
        "Repeat",
        "மீண்டும்",
        "ആവർത്തിക്കുക",
        "పునరావృతం",
        "दोहराएं"
    ),
    t!(
        "job.repeat.none",
        "Does not repeat",
        "மீண்டும் இல்லை",
        "ആവർത്തിക്കുന്നില്ല",
        "పునరావృతం లేదు",
        "दोहराव नहीं"
    ),
    t!(
        "job.repeat.seconds",
        "Every n seconds",
        "ஒவ்வொரு n விநாடிகளும்",
        "ഓരോ n സെക്കൻഡിലും",
        "ప్రతి n సెకన్లకు",
        "हर n सेकंड में"
    ),
    t!(
        "job.repeat.minutes",
        "Every n minutes",
        "ஒவ்வொரு n நிமிடங்களும்",
        "ഓരോ n മിനിറ്റിലും",
        "ప్రతి n నిమిషాలకు",
        "हर n मिनट में"
    ),
    t!(
        "job.repeat.days",
        "Every n days",
        "ஒவ்வொரு n நாட்களும்",
        "ഓരോ n ദിവസത്തിലും",
        "ప్రతి n రోజులకు",
        "हर n दिन में"
    ),
    t!(
        "job.repeat.weeks",
        "Every n weeks",
        "ஒவ்வொரு n வாரங்களும்",
        "ഓരോ n ആഴ്ചയിലും",
        "ప్రతి n వారాలకు",
        "हर n सप्ताह में"
    ),
    t!(
        "job.repeat.months",
        "Every n months",
        "ஒவ்வொரு n மாதங்களும்",
        "ഓരോ n മാസത്തിലും",
        "ప్రతి n నెలలకు",
        "हर n महीने में"
    ),
    t!(
        "job.repeat.weekday",
        "On a weekday",
        "வார நாளில்",
        "ആഴ്ചയിലെ ഒരു ദിവസം",
        "వారంలోని రోజున",
        "सप्ताह के दिन"
    ),
    t!(
        "job.repeat.day_of_month",
        "On the nth day of the month",
        "மாதத்தின் n-ஆம் நாள்",
        "മാസത്തിലെ n-ാം ദിവസം",
        "నెలలో n-వ రోజున",
        "महीने के n-वें दिन"
    ),
    t!(
        "job.history",
        "Run history",
        "இயக்க வரலாறு",
        "റൺ ചരിത്രം",
        "రన్ చరిత్ర",
        "रन इतिहास"
    ),
    t!("job.status", "Status", "நிலை", "നില", "స్థితి", "स्थिति"),
    t!("common.save", "Save", "சேமி", "സേവ് ചെയ്യുക", "సేవ్ చేయి", "सहेजें"),
    t!(
        "common.cancel",
        "Cancel",
        "ரத்து செய்",
        "റദ്ദാക്കുക",
        "రద్దు చేయి",
        "रद्द करें"
    ),
    t!(
        "common.delete",
        "Delete",
        "நீக்கு",
        "ഇല്ലാതാക്കുക",
        "తొలగించు",
        "हटाएं"
    ),
    t!(
        "common.prev",
        "Previous",
        "முந்தைய",
        "മുമ്പത്തേത്",
        "మునుపటి",
        "पिछला"
    ),
    t!("common.next", "Next", "அடுத்தது", "അടുത്തത്", "తదుపరి", "अगला"),
    t!(
        "settings.title",
        "Settings",
        "அமைப்புகள்",
        "ക്രമീകരണങ്ങൾ",
        "సెట్టింగ్‌లు",
        "सेटिंग्स"
    ),
    t!(
        "settings.full_name",
        "Full name",
        "முழு பெயர்",
        "പൂർണ്ണ നാമം",
        "పూర్తి పేరు",
        "पूरा नाम"
    ),
    t!("settings.theme", "Theme", "தீம்", "തീം", "థీమ్", "थीम"),
    t!("settings.theme.light", "Light", "ஒளி", "ലൈറ്റ്", "లైట్", "लाइट"),
    t!("settings.theme.dark", "Dark", "இருள்", "ഡാർക്ക്", "డార్క్", "डार्क"),
    t!(
        "settings.theme.system",
        "System",
        "அமைப்பு",
        "സിസ്റ്റം",
        "సిస్టమ్",
        "सिस्टम"
    ),
    t!(
        "settings.tab_orientation",
        "Category tabs",
        "வகை தாவல்கள்",
        "വിഭാഗ ടാബുകൾ",
        "వర్గ ట్యాబ్‌లు",
        "श्रेणी टैब"
    ),
    t!(
        "settings.horizontal",
        "Horizontal",
        "கிடைமட்டம்",
        "തിരശ്ചീനം",
        "అడ్డంగా",
        "क्षैतिज"
    ),
    t!(
        "settings.vertical",
        "Vertical",
        "செங்குத்து",
        "ലംബം",
        "నిలువుగా",
        "ऊर्ध्वाधर"
    ),
    t!(
        "settings.job_layout",
        "Job layout",
        "பணி அமைப்பு",
        "ജോലി ലേഔട്ട്",
        "ఉద్యోగ లేఅవుట్",
        "कार्य लेआउट"
    ),
    t!("settings.grid", "Grid", "கட்டம்", "ഗ്രിഡ്", "గ్రిడ్", "ग्रिड"),
    t!("settings.list", "List", "பட்டியல்", "ലിസ്റ്റ്", "జాబితా", "सूची"),
    t!(
        "settings.date_format",
        "Date format",
        "தேதி வடிவம்",
        "തീയതി ഫോർമാറ്റ്",
        "తేదీ ఫార్మాట్",
        "तिथि प्रारूप"
    ),
    t!(
        "settings.time_format",
        "Time format",
        "நேர வடிவம்",
        "സമയ ഫോർമാറ്റ്",
        "సమయ ఫార్మాట్",
        "समय प्रारूप"
    ),
    t!(
        "settings.page_size",
        "Rows per page",
        "ஒரு பக்கத்தில் வரிசைகள்",
        "ഒരു പേജിലെ വരികൾ",
        "పేజీకి వరుసలు",
        "प्रति पृष्ठ पंक्तियाँ"
    ),
    t!("settings.language", "Language", "மொழி", "ഭാഷ", "భాష", "भाषा"),
    t!(
        "settings.system_default",
        "System default",
        "கணினி இயல்பு",
        "സിസ്റ്റം ഡിഫോൾട്ട്",
        "సిస్టమ్ డిఫాల్ట్",
        "सिस्टम डिफ़ॉल्ट"
    ),
    t!(
        "settings.change_password",
        "Change password",
        "கடவுச்சொல்லை மாற்று",
        "പാസ്‌വേഡ് മാറ്റുക",
        "పాస్‌వర్డ్ మార్చండి",
        "पासवर्ड बदलें"
    ),
    t!(
        "settings.passkeys",
        "Passkeys",
        "கடவுச்சாவிகள்",
        "പാസ്‌കീകൾ",
        "పాస్‌కీలు",
        "पासकी"
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_cell_is_filled_and_keys_unique() {
        for (i, (k, cols)) in TABLE.iter().enumerate() {
            assert!(
                cols.iter().all(|c| !c.is_empty()),
                "empty translation for {k}"
            );
            assert!(TABLE[..i].iter().all(|(o, _)| o != k), "duplicate key {k}");
        }
    }
    #[test]
    fn lookup_and_fallback() {
        assert_eq!(tr(Language::En, "auth.login"), "Log in");
        assert_eq!(tr(Language::Hi, "auth.login"), "लॉग इन");
        assert_eq!(tr(Language::Ta, "missing.key"), "");
        assert_eq!(Language::parse("xx"), Language::En);
    }
}
