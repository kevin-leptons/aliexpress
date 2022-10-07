use crate::javascript::get_rhs_object;

#[test]
fn test_get_rhs_object() {
    let variable_name = "let b = ";
    let script = r#"
        let a = 123
        let b = {
            one: {
                two: 2
            }
        }
        let c = 789
        let b = {
            three: {
                four: "four"
            }
        }
    "#;
    let rhs_list = get_rhs_object(variable_name, script, 2);
    assert_eq!(rhs_list.len(), 2);
    assert_eq!(
        rhs_list[0].replace(" ", "").replace("\n", ""),
        "{one:{two:2}}".to_string()
    );
    assert_eq!(
        rhs_list[1].replace(" ", "").replace("\n", ""),
        "{three:{four:\"four\"}}".to_string()
    );
}
