use spooky_hash::SpookyHash;
fn main() {
    let mut h = SpookyHash::new(0, 0);
    let (h1, h2) = h.finalize();
    println\!(SpookyHash no updates: h1={:016x}, h2={:016x}, h1, h2);
    
    let mut h2_ = SpookyHash::new(0, 0);
    h2_.update(&[]);
    let (h1b, h2b) = h2_.finalize();
    println\!(SpookyHash empty update: h1={:016x}, h2={:016x}, h1b, h2b);
}
