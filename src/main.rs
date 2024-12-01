fn main() {
    pollster::block_on(mandelbrot::run());
}
