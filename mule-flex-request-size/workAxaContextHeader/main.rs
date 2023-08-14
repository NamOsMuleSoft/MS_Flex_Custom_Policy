// Function to calculate the factorial of a number
fn factorial(n: u64) -> u64 {
    if n == 0 {
        1
    } else {
        n * factorial(n - 1)
    }
}

fn main() {
    // Test cases
    let number1 = 5;
    let number2 = 10;
    let number3 = 0;

    let result1 = factorial(number1);
    let result2 = factorial(number2);
    let result3 = factorial(number3);

    println!("Factorial of {} is: {}", number1, result1);
    println!("Factorial of {} is: {}", number2, result2);
    println!("Factorial of {} is: {}", number3, result3);
}
