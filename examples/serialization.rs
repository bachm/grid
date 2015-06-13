extern crate grid;
extern crate rustc_serialize;
extern crate bincode;

fn main() {
    let mut original = grid::DynArray2::new(3, 2, 0u8);
    for (i, e) in original.iter_mut().enumerate() {
        *e = i as u8;
    }
    println!("Original: {:?}", original);
    let encoded: Vec<u8> = bincode::encode(&original, bincode::SizeLimit::Infinite).unwrap();

    let decoded: grid::DynArray2<u8> = bincode::decode(&encoded[..]).unwrap();
    println!("From bincode: {:?}", decoded);
    
    assert_eq!(original, decoded);
}