use std::{env, sync::OnceLock};

pub const TOKEN_EXPIRY: u64 = 604800; // secs
pub const EMAIL_TOKEN_EXPIRY: u64 = 900; // secs

pub const USERNAME_LIMIT: usize = 20;
pub const BIO_LIMIT: usize = 800;
pub const DISPLAY_NAME_LIMIT: usize = 30;

pub const MAX_PFP_WIDTH: u32 = 512;
pub const MAX_PFP_HEIGHT: u32 = 512;

pub const ASSETS_BUCKET: &'static str = "assets";
pub const PFPS_BUCKET: &'static str = "pfps";

// bytes
pub const ASSET_LIMIT: u64 = 15_000_000;
pub const PFP_LIMIT: u64 = 5_000_000;

pub const ALLOWED_IMAGE_HOSTS: [&'static str; 5] = [
    "u.cubeupload.com",
    "rdr.lol",
    "i.ibb.co",
    "i.imgur.com",
    "hatch.lol",
];

pub const COUNTRIES: [&'static str; 252] = [
    "United States",
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
    "South Georgia and the South Sandwish Islands",
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

pub const VERIFICATION_TEMPLATE: &'static str = r#"
<body style="background-color:#f9f9f9;">
<div style="margin:0px auto;max-width:600px;font-family:Helvetica Neue,Helvetica,Arial,Lucida Grande,sans-serif;font-size:16px;line-height:24px">
<img src="https://ci3.googleusercontent.com/meips/ADKq_NYSN-XcBSJFc0x3Picm27dUi35wi71cd_pbKriQt64Jsx3pIN4Hp-ZwmlOPdwgKKqUg=s0-d-e1-ft#https://rdr.lol/u/cVuLWI.png" height="50">
<hr>
<div style="background-color:white;padding: 10px;color: black !important;">
<h2>Welcome to hatch.lol!</h2>
<p>Hello {{username}}, thanks for joining</p>
<p>But before anything cool happens, please verify your email address:</p>
<center>
    <a href="{{link}}" target="_blank" style="background:linear-gradient(#FFBD59, #FDD18F);color:black;text-decoration:none;padding:10px 35px;font-weight:bold;">Verify</a>
</center>
<p style="color:grey"><small>Or use this link if that doesn't work: <a href="{{link}}" style="color:#D99E44">{{link}}</a></small></p>
</div>
</div>
</body>
"#;

pub fn postal_url() -> &'static str {
    static ADMIN_KEY: OnceLock<String> = OnceLock::new();
    ADMIN_KEY.get_or_init(|| env::var("POSTAL_URL").expect("POSTAL_URL not present"))
}

pub fn postal_key() -> &'static str {
    static ADMIN_KEY: OnceLock<String> = OnceLock::new();
    ADMIN_KEY.get_or_init(|| env::var("POSTAL_KEY").expect("POSTAL_KEY not present"))
}

pub fn logging_webhook() -> Option<&'static str> {
    static WEBHOOK: OnceLock<String> = OnceLock::new();
    let webhook_url =
        WEBHOOK.get_or_init(|| env::var("LOGGING_WEBHOOK").expect("LOGGING_WEBHOOK not present"));
    if webhook_url == "" {
        None
    } else {
        Some(&webhook_url)
    }
}

pub fn report_webhook() -> Option<&'static str> {
    static WEBHOOK: OnceLock<String> = OnceLock::new();
    let webhook_url =
        WEBHOOK.get_or_init(|| env::var("REPORT_WEBHOOK").expect("REPORT_WEBHOOK not present"));
    if webhook_url == "" {
        None
    } else {
        Some(&webhook_url)
    }
}

pub fn admin_key() -> &'static str {
    static ADMIN_KEY: OnceLock<String> = OnceLock::new();
    ADMIN_KEY.get_or_init(|| env::var("ADMIN_KEY").expect("ADMIN_KEY not present"))
}

pub fn base_url() -> &'static str {
    static BASE_URL: OnceLock<String> = OnceLock::new();
    BASE_URL.get_or_init(|| env::var("BASE_URL").expect("BASE_URL not present"))
}
