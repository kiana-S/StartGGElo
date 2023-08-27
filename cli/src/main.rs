use std::env;

fn get_auth_key() -> Option<String> {
    env::var("AUTH_KEY").ok()
}

fn main() {
    let _auth_key = get_auth_key().expect("Could not find authorization key");
}
