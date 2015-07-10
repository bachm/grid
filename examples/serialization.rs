extern crate grid;
extern crate rustc_serialize;
extern crate bincode;

fn main() {
    let mut count = 0;
    let original = grid::Array2::from_fn(3, 2, || {
        count += 1;
        count - 1
    });
    
    println!("Original:\n{:?}", original);
    
    let encoded: Vec<u8> = bincode::encode(&original, bincode::SizeLimit::Infinite).unwrap();

    let decoded: grid::Array2<u8> = bincode::decode(&encoded[..]).unwrap();
    println!("Decoded:\n{:?}", decoded);
    
    assert_eq!(original, decoded);
}