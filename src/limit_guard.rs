use rocket_governor::{Method, Quota, RocketGovernable};

pub struct OnePerMinute;

impl<'r> RocketGovernable<'r> for OnePerMinute {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::per_minute(Self::nonzero(1u32))
    }
}
pub struct TenPerSecond;

impl<'r> RocketGovernable<'r> for TenPerSecond {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::per_minute(Self::nonzero(1u32))
    }
}