#[cfg(test)]
mod test {
    use say_hi;

    #[test]
    fn should_say_hi() {
        assert_eq!(say_hi(), "hi");
    }
}
