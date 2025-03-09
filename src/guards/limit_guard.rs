use rocket_governor::{Method, Quota, RocketGovernable};

pub struct OnePerSecond;

impl<'r> RocketGovernable<'r> for OnePerSecond {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::per_second(Self::nonzero(1))
    }
}
pub struct TenPerSecond;

impl<'r> RocketGovernable<'r> for TenPerSecond {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::per_second(Self::nonzero(10))
    }
}
