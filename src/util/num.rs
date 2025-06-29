use color_eyre::{Result, Section, eyre::eyre};

pub enum Number {
    Integer(i64),
    Float(f64),
}

pub fn parse_num(num: &str) -> Result<Number> {
    let num = num.replace("_", "");

    match str::parse::<i64>(&num) {
        Ok(integer) => Ok(Number::Integer(integer)),
        Err(int_err) => match str::parse::<f64>(&num) {
            Ok(float) => Ok(Number::Float(float)),
            Err(float_err) => Err(eyre!("Failed to parse number.")
                .error(int_err)
                .error(float_err)),
        },
    }
}
