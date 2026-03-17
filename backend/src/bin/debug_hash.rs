use argon2::password_hash::{PasswordHash, PasswordVerifier};
use argon2::Argon2;

fn main() {
    let hash = "$argon2id$v=19$m=19456,t=2,p=1$0RQFgNg+eIEJ2wT1CasIpA$n3RW666rGddtUPDMRWBXo5ec9/yPi+GLAEu221b60rQ";
    let password = "TestPass123!";

    match PasswordHash::new(hash) {
        Ok(parsed_hash) => {
            println!("Hash parsed successfully");
            let argon2 = Argon2::default();
            match argon2.verify_password(password.as_bytes(), &parsed_hash) {
                Ok(_) => println!("Password verified!"),
                Err(e) => println!("Verification failed: {}", e),
            }
        }
        Err(e) => println!("Hash parsing failed: {}", e),
    }
}
