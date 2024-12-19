// Use wit_bindgen to generate the bindings from the component model to Rust.
// For more information see: https://github.com/bytecodealliance/wit-bindgen/
wit_bindgen::generate!({
    path: "..",
    world: "convert",
});

struct GuestComponent;

export!(GuestComponent);

impl Guest for GuestComponent {
    fn convert_celsius_to_fahrenheit(x: f32) -> f32 {
        host::apply(
            host::apply(x, 1.8, host::BinaryOperation::Multiply),
            32.0,
            host::BinaryOperation::Add,
        )
    }

    fn convert(t: Temperature) -> Temperature {
        match t {
            Temperature::Celsius(t) => Temperature::Fahrenheit(host::multiply(t, 1.8) + 32.0),
            Temperature::Fahrenheit(t) => Temperature::Celsius(host::multiply(t - 32.0, 5.0 / 9.0)),
        }
    }
}
