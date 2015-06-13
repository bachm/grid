extern crate grid;

fn main() {
    let mut grid = grid::DynArray2::new(3, 3, 0u8);
    for (i, e) in grid.iter_mut().enumerate() {
        *e = i as u8;
    }
    
    println!("The entire grid:");
    for row in grid.rows() {
        println!("{:?}", row);
    }
    
    let x: u16 = 1;
    let y: u16 = 1;
    let width: u16 = 4;
    let height: u16 = 4;    
    println!("\nA view into the grid from {},{} to {},{}:", x, y, x + width, y + height);
    for row in grid.view(x, y, width, height) {
        println!("{:?}", row);
    }
}