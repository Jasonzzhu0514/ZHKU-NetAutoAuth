// Read-only campus diagnostics; prints no account or device identifiers.
#[allow(dead_code)]
#[path = "../src/portal.rs"]
mod portal;

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
