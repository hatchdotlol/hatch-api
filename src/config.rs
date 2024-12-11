pub const TOKEN_EXPIRY: u64 = 1000; // secs

pub const USERNAME_LIMIT: usize = 15;
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
