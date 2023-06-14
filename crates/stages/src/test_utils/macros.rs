macro_rules! stage_test_suite {
    ($runner:ident, $name:ident) => {

         paste::item! {
            /// Check that the execution is short-circuited if the database is empty.
            #[tokio::test]
            async fn [< execute_empty_db_ $name>] () {
                // Set up the runner
                let runner = $runner::default();

                // Execute the stage with empty database
                let input = crate::stage::ExecInput::default();

                // Run stage execution
                let result = runner.execute(input).await;
                // Check that the result is returned and the stage does not panic.
                // The return result with empty db is stage-specific.
                assert_matches::assert_matches!(result, Ok(_));

                // Validate the stage execution
                assert!(runner.validate_execution(input, result.unwrap().ok()).is_ok(), "execution validation");
            }

            // Run the complete stage execution flow.
            #[tokio::test]
            async fn [< execute_ $name>] () {
                let (previous_stage, stage_progress) = (500, 100);

                // Set up the runner
                let mut runner = $runner::default();
                let input = crate::stage::ExecInput {
                    target: Some(previous_stage),
                    checkpoint: Some(reth_primitives::stage::StageCheckpoint::new(stage_progress)),
                };
                let seed = runner.seed_execution(input).expect("failed to seed");
                let db = runner.tx().inner_raw();
                let factory = ProviderFactory::new(db.as_ref(), MAINNET.clone());

                let rx = runner.execute(input);

                // Run `after_execution` hook
                runner.after_execution(seed).await.expect("failed to run after execution hook");

                // Assert the successful result
                let result = rx.await.unwrap();
                assert_matches::assert_matches!(result, Ok(_));

                let output = result.unwrap();
                assert!(runner.stage().is_execute_done(&mut factory.provider_rw().unwrap(), input, output).await.unwrap());
                assert_matches::assert_matches!(
                    output,
                    ExecOutput { checkpoint } if checkpoint.block_number == previous_stage
                );

                // Validate the stage execution
                assert!(runner.validate_execution(input, Some(output)).is_ok(), "execution validation");
            }

            // Check that unwind does not panic on no new entries within the input range.
            #[tokio::test]
            async fn [< unwind_no_new_entries_ $name>] () {
                // Set up the runner
                let mut runner = $runner::default();
                let input = crate::stage::UnwindInput::default();

                // Seed the database
                runner.seed_execution(crate::stage::ExecInput::default()).expect("failed to seed");

                runner.before_unwind(input).expect("failed to execute before_unwind hook");

                // Run stage unwind
                let rx = runner.unwind(input).await;
                assert_matches::assert_matches!(
                    rx,
                    Ok(UnwindOutput { checkpoint }) if checkpoint.block_number == input.unwind_to
                );

                // Validate the stage unwind
                assert!(runner.validate_unwind(input).is_ok(), "unwind validation");
            }

            // Run complete execute and unwind flow.
            #[tokio::test]
            async fn [< unwind_ $name>] () {
                let (previous_stage, stage_progress) = (500, 100);

                // Set up the runner
                let mut runner = $runner::default();
                let execute_input = crate::stage::ExecInput {
                    target: Some(previous_stage),
                    checkpoint: Some(reth_primitives::stage::StageCheckpoint::new(stage_progress)),
                };
                let seed = runner.seed_execution(execute_input).expect("failed to seed");
                let db = runner.tx().inner_raw();
                let factory = ProviderFactory::new(db.as_ref(), MAINNET.clone());

                // Run stage execution
                let rx = runner.execute(execute_input);
                runner.after_execution(seed).await.expect("failed to run after execution hook");

                // Assert the successful execution result
                let stage = runner.stage();

                let result = rx.await.unwrap();
                assert_matches::assert_matches!(result, Ok(_));

                let execute_output = result.unwrap();
                assert!(stage.is_execute_done(&mut factory.provider_rw().unwrap(), execute_input, execute_output).await.unwrap());
                assert_matches::assert_matches!(
                    execute_output,
                    ExecOutput { checkpoint } if checkpoint.block_number == previous_stage
                );
                assert!(runner.validate_execution(execute_input, Some(execute_output)).is_ok(), "execution validation");

                // Run stage unwind
                let unwind_input = crate::stage::UnwindInput {
                    unwind_to: stage_progress,
                    checkpoint: reth_primitives::stage::StageCheckpoint::new(previous_stage),
                    bad_block: None,
                };

                runner.before_unwind(unwind_input).expect("Failed to unwind state");

                let rx = runner.unwind(unwind_input).await;
                assert_matches::assert_matches!(rx, Ok(_));
                let unwind_output = rx.unwrap();

                // Assert the successful unwind result
                assert!(stage.is_unwind_done(&mut factory.provider_rw().unwrap(), unwind_input, unwind_output));
                assert_matches::assert_matches!(
                    unwind_output,
                    UnwindOutput { checkpoint } if checkpoint.block_number == unwind_input.unwind_to
                );

                // Validate the stage unwind
                assert!(runner.validate_unwind(unwind_input).is_ok(), "unwind validation");
            }
        }
    };
}

pub(crate) use stage_test_suite;
