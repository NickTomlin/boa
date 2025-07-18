use crate::{TestAction, run_test_actions};
use indoc::indoc;

#[test]
fn promise() {
    run_test_actions([
        TestAction::run(indoc! {r#"
                    let count = 0;
                    const promise = new Promise((resolve, reject) => {
                        count += 1;
                        resolve(undefined);
                    }).then((_) => (count += 1));
                    count += 1;
                "#}),
        TestAction::assert_eq("count", 2),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("count", 3),
    ]);
}
