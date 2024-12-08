#[macro_use]
extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    "hey vsauce michael here do you want to see the most illegal thing i own?\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\nits a penny"
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
