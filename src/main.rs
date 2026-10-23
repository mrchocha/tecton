use storage::engine::{Storage, StorageOptions};

fn main() {
    let options = StorageOptions::new();
    let mut storage = Storage::new(options).expect("Error while init storage");

    storage
        .put("Rahul".to_string(), "Ahir".to_string())
        .expect("Error while inserting key");

    storage.flush().expect("error while flush");
    println!("Hello, world! ");
}
