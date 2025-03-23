use std::{collections::HashMap, env, sync::OnceLock};

pub static TOKEN_EXPIRY: u64 = 604800; // secs
pub static EMAIL_TOKEN_EXPIRY: u64 = 900; // secs
pub static USER_DELETION: u64 = 259200; // secs

pub static USERNAME_LIMIT: usize = 20;
pub static BIO_LIMIT: usize = 800;
pub static DISPLAY_NAME_LIMIT: usize = 30;

pub static MAX_PFP_WIDTH: u32 = 512;
pub static MAX_PFP_HEIGHT: u32 = 512;

pub static PROJECTS_BUCKET: &str = "projects";
pub static PFPS_BUCKET: &str = "pfps";
pub static THUMBNAILS_BUCKET: &str = "thumbnails";

// bytes
pub static ASSET_LIMIT: u64 = 15_000_000;
pub static PFP_LIMIT: u64 = 5_000_000;
pub static TITLE_LIMIT: u64 = 64;
pub static DESCRIPTION_LIMIT: u64 = 5000;

pub static ALLOWED_IMAGE_HOSTS: [&str; 5] = [
    "u.cubeupload.com",
    "rdr.lol",
    "i.ibb.co",
    "i.imgur.com",
    "hatch.lol",
];

pub static COUNTRIES: [&str; 252] = [
    "Location Not Given",
    "Afghanistan",
    "Åland Islands",
    "Algeria",
    "American Samoa",
    "Andorra",
    "Angola",
    "Anguilla",
    "Antarctica",
    "Antigua and Barbuda",
    "Argentina",
    "Armenia",
    "Aruba",
    "Ascension Island",
    "Australia",
    "Austria",
    "Azerbaijan",
    "The Bahamas",
    "Bahrain",
    "Bangladesh",
    "Barbados",
    "Belarus",
    "Belgium",
    "Belize",
    "Benin",
    "Bermuda",
    "Bhutan",
    "Bolivia",
    "Bonaire",
    "Bosnia and Herzegovina",
    "Botswana",
    "Bouvet Island",
    "Brazil",
    "British Indian Ocean Territory",
    "British Virgin Islands",
    "Brunei",
    "Bulgaria",
    "Burkina Faso",
    "Cambodia",
    "Cameroon",
    "Canada",
    "Cape Verde",
    "The Cayman Islands",
    "Central African Republic",
    "Chad",
    "China",
    "Christmas Island",
    "Cocos (Keeling) Islands",
    "Colombia",
    "Comoros",
    "Democratic Republic of the Congo",
    "Republic of Congo",
    "Cook Islands",
    "Costa Rica",
    "Côte d'Ivoire",
    "Croatia",
    "Cuba",
    "Curaçao",
    "Cyprus",
    "Czechia",
    "Denmark",
    "Djibouti",
    "Dominica",
    "Dominican Republic",
    "Ecuador",
    "Egypt",
    "El Salvador",
    "Equatorial Guinea",
    "Eritrea",
    "Estonia",
    "Eswatini",
    "Ethiopia",
    "Falkland Islands",
    "Faroe Islands",
    "Fiji",
    "Finland",
    "France",
    "French Guiana",
    "French Polynesia",
    "French Southern Territory",
    "Gabon",
    "The Gambia",
    "Georgia",
    "Germany",
    "Ghana",
    "Gibraltar",
    "Greece",
    "Greenland",
    "Grenada",
    "Guadeloupe",
    "Guam",
    "Guatemala",
    "Guernsey",
    "Guinea",
    "Guinea-Bissau",
    "Guyana",
    "Haiti",
    "Heard Island and McDonald Island",
    "Honduras",
    "Hong Kong",
    "Hungary",
    "Iceland",
    "India",
    "Indonesia",
    "Iran",
    "Iraq",
    "Ireland",
    "Isle of Man",
    "Israel",
    "Italy",
    "Jamaica",
    "Jan Mayen",
    "Japan",
    "Jersey",
    "Jordan",
    "Kazakhstan",
    "Kenya",
    "Kiribati",
    "Kuwait",
    "Kyrgyzsran",
    "Laos",
    "Latvia",
    "Lebanon",
    "Lesotho",
    "Liberia",
    "Libya",
    "Liechtenstein",
    "Lithuania",
    "Luxembourg",
    "Macao",
    "Madagascar",
    "Malawi",
    "Malaysia",
    "Maldives",
    "Mali",
    "Malta",
    "Marshall Islands",
    "Martinique",
    "Mauritania",
    "Mauritius",
    "Mayotte",
    "Mexico",
    "Micronesia",
    "Moldova",
    "Monaco",
    "Mongolia",
    "Montenegro",
    "Montserrat",
    "Morocco",
    "Mozambique",
    "Myanmar",
    "Namibia",
    "Nauru",
    "Nepal",
    "Netherlands",
    "New Caledonia",
    "New Zealand",
    "Nicaragua",
    "Niger",
    "Nigeria",
    "Niue",
    "Norfolk Island",
    "North Korea",
    "North Macedonia",
    "Northern Mariana Islands",
    "Norway",
    "Oman",
    "Pakistan",
    "Palau",
    "Palestine",
    "Panama",
    "Papua New Guinea",
    "Paraguay",
    "Peru",
    "The Philippines",
    "Pitcairn Islands",
    "Poland",
    "Portugal",
    "Puerto Rico",
    "Qatar",
    "Réunion",
    "Romania",
    "Russia",
    "Rwanda",
    "Saba",
    "Samoa",
    "San Marino",
    "São Tomé and Príncipe",
    "Saudi Arabia",
    "Senegal",
    "Serbia",
    "Seychelles",
    "Sierra Leone",
    "Singapore",
    "Sint Eustatius",
    "Sint Maarten",
    "Slovakia",
    "Slovenia",
    "Solomon Islands",
    "Somalia",
    "South Africa",
    "South Georgia and the South Sandwich Islands",
    "South Korea",
    "South Sudan",
    "Spain",
    "Sri Lanka",
    "St. Barthélemy",
    "St. Helena",
    "St. Kitts and Nevis",
    "St. Lucia",
    "St. Martin",
    "St. Pierre and Miquelon",
    "St. Vincent and the Grenadines",
    "Sudam",
    "Suriname",
    "Svalbard",
    "Sweden",
    "Switzerland",
    "Syria",
    "Taiwan",
    "Tajikistan",
    "Tanzania",
    "Thailand",
    "Timor-Leste",
    "Togo",
    "Tokelau",
    "Tonga",
    "Trinidad and Tobago",
    "Tristan da Cunha",
    "Tunisia",
    "Türkiye",
    "Turkmenistan",
    "Turks and Caicos Islands",
    "Tuvalu",
    "Uganda",
    "Ukraine",
    "United Arab Emirates",
    "United Kingdom",
    "United States Minor Outlying Islands",
    "United States",
    "Tunisia",
    "Uruguay",
    "Uzbekistan",
    "Vanuatu",
    "Vatican City",
    "Venezuela",
    "Vietnam",
    "Wallis and Futuna",
    "Western Sahara",
    "Yemen",
    "Zambia",
    "Zimbabwe",
];

pub static VERIFICATION_TEMPLATE: &str = r#"
<body style="background-color:#f9f9f9;">
<div style="margin:0px auto;max-width:600px;font-family:Helvetica Neue,Helvetica,Arial,Lucida Grande,sans-serif;font-size:16px;line-height:24px">
<img src="https://rdr.lol/u/qEJWct.png" height="50">
<hr>
<div style="background-color:white;padding: 10px;color: black !important;">
<h2>Welcome to hatch.lol!</h2>
<p>Hello {{username}}, thanks for joining</p>
<p>But before anything cool happens, please verify your email address:</p>
<center>
    <a href="{{link}}" target="_blank" style="background:linear-gradient(#FFBD59, #FDD18F);color:black;text-decoration:none;padding:10px 35px;font-weight:bold;">Verify</a>
</center>
<p style="color:grey"><small>Or use this link if that doesn't work: <a href="{{link}}" style="color:#D99E44">{{link}}</a></small></p>
<p>If you didn't create this account, please ignore this email.</p>
</div>
</div>
</body>
"#;

pub struct Config {
    pub mods: HashMap<String, bool>,
    pub postal_url: String,
    pub postal_key: String,
    pub backup_resend_key: Option<String>,
    pub logging_webhook: Option<String>,
    pub report_webhook: Option<String>,
    pub admin_key: String,
    pub base_url: String,
    pub minio_url: String,
    pub minio_access_key: String,
    pub minio_secret_key: String,
    pub db_path: String,
}

pub fn config() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();

    CONFIG.get_or_init(|| {
        let mut mods = HashMap::new();
        let mod_str = env::var("MODS").expect("MODS not present");

        for moderator in mod_str.split(",") {
            mods.insert(moderator.into(), true);
        }

        let postal_url = env::var("POSTAL_URL").expect("POSTAL_URL not present");
        let postal_key = env::var("POSTAL_KEY").expect("POSTAL_KEY not present");

        let backup_resend_key = env::var("RESEND_KEY").unwrap_or("".into());
        let backup_resend_key = if backup_resend_key == "" {
            None
        } else {
            Some(backup_resend_key)
        };

        let logging_webhook = env::var("LOGGING_WEBHOOK").unwrap_or("".into());
        let logging_webhook = if logging_webhook == "" {
            None
        } else {
            Some(logging_webhook)
        };

        let report_webhook = env::var("REPORT_WEBHOOK").unwrap_or("".into());
        let report_webhook = if report_webhook == "" {
            None
        } else {
            Some(report_webhook)
        };

        let admin_key = env::var("ADMIN_KEY").expect("ADMIN_KEY not present");
        let base_url = env::var("BASE_URL").expect("BASE_URL not present");
        let minio_url = env::var("MINIO_URL").expect("MINIO_URL not present");
        let minio_access_key = env::var("MINIO_ACCESS_KEY").expect("MINIO_ACCESS_KEY not present");
        let minio_secret_key = env::var("MINIO_SECRET_KEY").expect("MINIO_SECRET_KEY not present");
        let db_path = env::var("DB_PATH").expect("DB_PATH not present");

        Config {
            mods,
            postal_url,
            postal_key,
            backup_resend_key,
            logging_webhook,
            report_webhook,
            admin_key,
            base_url,
            minio_url,
            minio_access_key,
            minio_secret_key,
            db_path,
        }
    })
}
