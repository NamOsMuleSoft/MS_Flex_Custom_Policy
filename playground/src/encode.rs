use rsa::{RSAPrivateKey, PaddingScheme};

fn main() {
    // Replace these values with your actual RSA private key components
    let modulus: Vec<u8> = vec![/* Insert your RSA modulus bytes here */];
    let private_exponent: Vec<u8> = vec![/* Insert your RSA private exponent bytes here */];

    let private_key = RSAPrivateKey::from_components(modulus.clone(), private_exponent.clone())
        .expect("Failed to create private key");

    let payload_base64 = "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJpc..."; // Your base64-encoded payload

    let payload_bytes = base64::decode(payload_base64).expect("Failed to decode payload");
    
    let padding = PaddingScheme::PKCS1v15Sign {
        hash: None, // No need to specify the hash here
    };

    let signature = private_key.sign(padding, &payload_bytes)
        .expect("Failed to sign payload");

    println!("Signature: {:?}", signature);
}
