// Read-only campus diagnostics; prints no account or device identifiers.
#[allow(dead_code)]
#[path = "../src/portal.rs"]
mod portal;
#[path = "../src/portal_format.rs"]
mod portal_format;

fn main() {
    match portal::Portal::new() {
        Ok(client) => {
            let status = client.status();
            println!(
                "region={} state={:?} detail={}",
                status.region, status.state, status.detail
            );
        }
        Err(error) => println!("{error}"),
    }
}
