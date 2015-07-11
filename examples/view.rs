extern crate grid;

fn main() {
    let mut n = 0u8;
    let mut grid = grid::Array2::from_fn(3, 3, || { n += 1; n - 1 });
    
    println!("Rows:");
    for row in grid.rows() {
        println!("{:?}", row);
    }
    
    let x = 1;
    let y = 1;
    let width = 2;
    let height = 2;    
    println!("\nA view into a subsection of the array (from {},{} to {},{} exclusive):", x, y, x + width, y + height);
    for slice in grid.view(x, y, width, height) {
        println!("{:?}", slice);
    }
}