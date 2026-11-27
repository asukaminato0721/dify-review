# Backtest

625 chunks (a file's hunks, judged together as review does). Positives: chunks the reviewer commented on, labelled in `backtest/labels.toml`. Negatives: chunks in the final diff of the merged pull requests those comments come from. *hits*: positives the rule would have commented on; *fix*: of those, with the labelled option; *noise*: negatives without the rule's hint pattern it would have commented on; *tolerated*: negatives that do contain the pattern (the reviewer let it through at the time) it would have commented on. Precision and the suggestion (max F0.5) count noise only.

## avoid-cast

11 positives (0 outside the rule's paths), 275 negatives, 0 live comments judged wrong; threshold now 0.35, suggested 0.35.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 11/11 | 100% | 0 | 100% | 3 |
| 0.35 | 11/11 | 100% | 0 | 100% | 3 |
| 0.40 | 10/11 | 100% | 0 | 100% | 3 |
| 0.45 | 10/11 | 100% | 0 | 100% | 3 |
| 0.50 | 10/11 | 100% | 0 | 100% | 3 |
| 0.55 | 10/11 | 100% | 0 | 100% | 3 |
| 0.60 | 10/11 | 100% | 0 | 100% | 3 |
| 0.65 | 10/11 | 100% | 0 | 100% | 2 |
| 0.70 | 10/11 | 100% | 0 | 100% | 2 |
| 0.75 | 10/11 | 100% | 0 | 100% | 2 |
| 0.80 | 10/11 | 100% | 0 | 100% | 2 |
| 0.85 | 9/11 | 100% | 0 | 100% | 2 |
| 0.90 | 9/11 | 100% | 0 | 100% | 2 |
| 0.95 | 8/11 | 100% | 0 | 100% | 2 |

Tolerated: pattern present, reviewer let it through (3):

- https://github.com/langgenius/dify/pull/37950/files api/tests/unit_tests/controllers/openapi/test_contract.py `@@ -5,6 +5,7 @@`
- https://github.com/langgenius/dify/pull/36791/files api/tests/test_containers_integration_tests/controllers/console/workspace/test_members.py `@@ -1,9 +1,14 @@`
- https://github.com/langgenius/dify/pull/39822/files api/extensions/ext_application_services.py `@@ -0,0 +1,44 @@`

## inject-session

20 positives (0 outside the rule's paths), 65 negatives, 0 live comments judged wrong; threshold now 0.75, suggested 0.75.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 20/20 | 100% | 0 | 100% | 8 |
| 0.35 | 20/20 | 100% | 0 | 100% | 8 |
| 0.40 | 20/20 | 100% | 0 | 100% | 8 |
| 0.45 | 20/20 | 100% | 0 | 100% | 8 |
| 0.50 | 20/20 | 100% | 0 | 100% | 7 |
| 0.55 | 20/20 | 100% | 0 | 100% | 6 |
| 0.60 | 20/20 | 100% | 0 | 100% | 6 |
| 0.65 | 20/20 | 100% | 0 | 100% | 6 |
| 0.70 | 20/20 | 100% | 0 | 100% | 6 |
| 0.75 | 20/20 | 100% | 0 | 100% | 6 |
| 0.80 | 17/20 | 100% | 0 | 100% | 5 |
| 0.85 | 10/20 | 100% | 0 | 100% | 2 |
| 0.90 | 6/20 | 100% | 0 | 100% | 1 |
| 0.95 | 1/20 | 100% | 0 | 100% | 1 |

Tolerated: pattern present, reviewer let it through (6):

- https://github.com/langgenius/dify/pull/38280/files api/libs/pagination.py `@@ -0,0 +1,86 @@`
- https://github.com/langgenius/dify/pull/41007/files api/controllers/web/wraps.py `@@ -6,11 +6,11 @@`
- https://github.com/langgenius/dify/pull/42248/files api/controllers/common/rbac/locators.py `@@ -6,6 +6,7 @@`
- https://github.com/langgenius/dify/pull/42335/files api/core/memory/token_buffer_memory.py `@@ -64,7 +64,9 @@ def _build_prompt_message_with_files(`
- https://github.com/langgenius/dify/pull/42335/files api/core/ops/ops_trace_manager.py `@@ -1188,7 +1188,7 @@ def dataset_retrieval_trace(self, message_id, timer, **kwargs):`
- https://github.com/langgenius/dify/pull/42335/files api/models/model.py `@@ -1198,12 +1198,7 @@ class Conversation(Base):`

## no-bypass-init

4 positives (0 outside the rule's paths), 275 negatives, 0 live comments judged wrong; threshold now 0.90, suggested 0.90.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 2/4 | 100% | 0 | 100% | 0 |
| 0.35 | 2/4 | 100% | 0 | 100% | 0 |
| 0.40 | 2/4 | 100% | 0 | 100% | 0 |
| 0.45 | 2/4 | 100% | 0 | 100% | 0 |
| 0.50 | 2/4 | 100% | 0 | 100% | 0 |
| 0.55 | 2/4 | 100% | 0 | 100% | 0 |
| 0.60 | 2/4 | 100% | 0 | 100% | 0 |
| 0.65 | 2/4 | 100% | 0 | 100% | 0 |
| 0.70 | 2/4 | 100% | 0 | 100% | 0 |
| 0.75 | 2/4 | 100% | 0 | 100% | 0 |
| 0.80 | 2/4 | 100% | 0 | 100% | 0 |
| 0.85 | 2/4 | 100% | 0 | 100% | 0 |
| 0.90 | 2/4 | 100% | 0 | 100% | 0 |
| 0.95 | 1/4 | 100% | 0 | 100% | 0 |

Missed comments (2):

- https://github.com/langgenius/dify/pull/44775#discussion_r4374124655 (hint not matched in scope)
- https://github.com/langgenius/dify/pull/44837#discussion_r4424553072 (jev 0.17)

## no-reflection

11 positives (0 outside the rule's paths), 275 negatives, 0 live comments judged wrong; threshold now 0.90, suggested 0.90.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 10/11 | 100% | 7 | 59% | 13 |
| 0.35 | 10/11 | 100% | 4 | 71% | 12 |
| 0.40 | 10/11 | 100% | 2 | 83% | 11 |
| 0.45 | 10/11 | 100% | 2 | 83% | 11 |
| 0.50 | 10/11 | 100% | 2 | 83% | 9 |
| 0.55 | 10/11 | 100% | 2 | 83% | 8 |
| 0.60 | 10/11 | 100% | 1 | 91% | 7 |
| 0.65 | 9/11 | 100% | 1 | 90% | 6 |
| 0.70 | 9/11 | 100% | 1 | 90% | 5 |
| 0.75 | 9/11 | 100% | 1 | 90% | 5 |
| 0.80 | 9/11 | 100% | 1 | 90% | 5 |
| 0.85 | 9/11 | 100% | 1 | 90% | 5 |
| 0.90 | 9/11 | 100% | 0 | 100% | 4 |
| 0.95 | 8/11 | 100% | 0 | 100% | 2 |

Missed comments (2):

- https://github.com/langgenius/dify/pull/45458#discussion_r4464595002 (jev 0.61)
- https://github.com/langgenius/dify/pull/45458#discussion_r4464598580 (jev 0.20)

Tolerated: pattern present, reviewer let it through (4):

- https://github.com/langgenius/dify/pull/40379/files api/tests/unit_tests/controllers/console/workspace/test_snippets.py `@@ -1,14 +1,17 @@`
- https://github.com/langgenius/dify/pull/42034/files api/fields/base.py `@@ -1,7 +1,27 @@`
- https://github.com/langgenius/dify/pull/45633/files api/tests/unit_tests/commands/test_system.py `@@ -2,6 +2,7 @@`
- https://github.com/langgenius/dify/pull/46419/files api/tests/unit_tests/controllers/console/app/test_model_config.py `@@ -3,6 +3,7 @@`

## no-test-for-type-errors

14 positives (1 outside the rule's paths), 181 negatives, 2 live comments judged wrong; threshold now 0.95, suggested 0.95.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 10/14 | 100% | 34 | 23% | 0 |
| 0.35 | 10/14 | 100% | 24 | 29% | 0 |
| 0.40 | 10/14 | 100% | 23 | 30% | 0 |
| 0.45 | 10/14 | 100% | 19 | 34% | 0 |
| 0.50 | 10/14 | 100% | 18 | 36% | 0 |
| 0.55 | 10/14 | 100% | 16 | 38% | 0 |
| 0.60 | 10/14 | 100% | 15 | 40% | 0 |
| 0.65 | 8/14 | 100% | 12 | 40% | 0 |
| 0.70 | 7/14 | 100% | 10 | 41% | 0 |
| 0.75 | 7/14 | 100% | 9 | 44% | 0 |
| 0.80 | 5/14 | 100% | 6 | 45% | 0 |
| 0.85 | 4/14 | 100% | 5 | 44% | 0 |
| 0.90 | 2/14 | 100% | 5 | 29% | 0 |
| 0.95 | 1/14 | 100% | 1 | 50% | 0 |

Missed comments (13):

- https://github.com/langgenius/dify/pull/44567#discussion_r4611245164 (jev 0.11)
- https://github.com/langgenius/dify/pull/44567#discussion_r4611245247 (jev 0.09)
- https://github.com/langgenius/dify/pull/45180#discussion_r4430567006 (jev 0.67)
- https://github.com/langgenius/dify/pull/45648#discussion_r4488870073 (not asked)
- https://github.com/langgenius/dify/pull/45725#discussion_r4500515284 (jev 0.62)
- https://github.com/langgenius/dify/pull/45726#discussion_r4491689346 (jev 0.76)
- https://github.com/langgenius/dify/pull/45729#discussion_r4491655409 (jev 0.89)
- https://github.com/langgenius/dify/pull/45731#discussion_r4491669682 (jev 0.80)
- https://github.com/langgenius/dify/pull/45768#discussion_r4505007988 (jev 0.64)
- https://github.com/langgenius/dify/pull/45819#discussion_r4505665941 (jev 0.92)
- https://github.com/langgenius/dify/pull/45968#discussion_r4579156703 (jev 0.79)
- https://github.com/langgenius/dify/pull/46404#discussion_r4577205798 (jev 0.23)
- https://github.com/langgenius/dify/pull/46457#discussion_r4583995474 (jev 0.87)

Noise: would comment where the pattern is absent (1):

- https://github.com/langgenius/dify/pull/46463#discussion_r4609485912 (judged wrong live)

## no-type-ignore

21 positives (0 outside the rule's paths), 275 negatives, 1 live comments judged wrong; threshold now 0.75, suggested 0.75.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 21/21 | 100% | 2 | 91% | 18 |
| 0.35 | 21/21 | 100% | 2 | 91% | 18 |
| 0.40 | 21/21 | 100% | 1 | 95% | 18 |
| 0.45 | 21/21 | 100% | 1 | 95% | 18 |
| 0.50 | 21/21 | 100% | 1 | 95% | 18 |
| 0.55 | 21/21 | 100% | 1 | 95% | 18 |
| 0.60 | 21/21 | 100% | 1 | 95% | 18 |
| 0.65 | 21/21 | 100% | 1 | 95% | 18 |
| 0.70 | 21/21 | 100% | 1 | 95% | 17 |
| 0.75 | 21/21 | 100% | 1 | 95% | 17 |
| 0.80 | 20/21 | 100% | 1 | 95% | 16 |
| 0.85 | 14/21 | 100% | 1 | 93% | 14 |
| 0.90 | 9/21 | 100% | 0 | 100% | 8 |
| 0.95 | 3/21 | 100% | 0 | 100% | 0 |

Noise: would comment where the pattern is absent (1):

- https://github.com/langgenius/dify/pull/46648#discussion_r4611517451 (judged wrong live)

Tolerated: pattern present, reviewer let it through (17):

- https://github.com/langgenius/dify/pull/38280/files api/libs/pagination.py `@@ -0,0 +1,86 @@`
- https://github.com/langgenius/dify/pull/43112/files api/tests/unit_tests/controllers/console/app/test_workflow_draft_variable_handlers.py `@@ -0,0 +1,219 @@`
- https://github.com/langgenius/dify/pull/43112/files api/tests/unit_tests/controllers/console/datasets/rag_pipeline/test_rag_pipeline_draft_variable_handlers.py `@@ -0,0 +1,166 @@`
- https://github.com/langgenius/dify/pull/44815/files api/tests/unit_tests/libs/test_custom_inputs.py `@@ -8,61 +8,61 @@`
- https://github.com/langgenius/dify/pull/44458/files api/providers/vdb/vdb-alibabacloud-mysql/tests/unit_tests/test_alibabacloud_mysql_vector.py `@@ -360,8 +360,8 @@ def test_delete_by_ids_table_not_exists(self, mock_pool_class):`
- https://github.com/langgenius/dify/pull/44458/files api/providers/vdb/vdb-clickzetta/tests/unit_tests/test_clickzetta_vector.py `@@ -1,19 +1,19 @@`
- https://github.com/langgenius/dify/pull/44881/files api/tests/unit_tests/fields/test_dataset_fields.py `@@ -19,13 +19,15 @@`
- https://github.com/langgenius/dify/pull/44875/files api/tests/unit_tests/events/test___init__.py `@@ -3,40 +3,42 @@`
- https://github.com/langgenius/dify/pull/44875/files api/tests/unit_tests/events/test_app_event.py `@@ -5,11 +5,12 @@`
- https://github.com/langgenius/dify/pull/44885/files api/tests/unit_tests/libs/test_archive_storage.py `@@ -1,10 +1,13 @@`
- https://github.com/langgenius/dify/pull/44885/files api/tests/unit_tests/libs/test_external_api.py `@@ -6,7 +6,8 @@`
- https://github.com/langgenius/dify/pull/45320/files api/tests/unit_tests/core/rag/datasource/keyword/jieba/test_jieba_keyword_table_handler.py `@@ -1,140 +1,124 @@`
- https://github.com/langgenius/dify/pull/45336/files api/tests/unit_tests/core/app/app_config/easy_ui_based_app/model_config/test_manager.py `@@ -326,7 +326,7 @@ def test_validate_and_set_defaults_provider_without_slash_converted(`
- https://github.com/langgenius/dify/pull/45336/files api/tests/unit_tests/core/workflow/nodes/agent_v2/test_session_store.py `@@ -158,7 +158,14 @@ def _install_backend_client(monkeypatch: pytest.MonkeyPatch) -> MagicMock:`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/console/app/test_workflow_run.py `@@ -152,8 +152,8 @@ def test_pause_details_response_schema_is_registered() -> None:`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/console/tag/test_tags.py `@@ -1,3 +1,5 @@`
- https://github.com/langgenius/dify/pull/45766/files api/tests/unit_tests/core/plugin/impl/test_model_runtime_factory.py `@@ -405,7 +405,7 @@ def test_create_model_type_instance_builds_model_wrappers(`

## parse-dont-validate

114 positives (0 outside the rule's paths), 275 negatives, 3 live comments judged wrong; threshold now 0.30, suggested 0.30.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 109/114 | 100% | 3 | 97% | 41 |
| 0.35 | 107/114 | 100% | 3 | 97% | 38 |
| 0.40 | 96/114 | 100% | 3 | 97% | 33 |
| 0.45 | 88/114 | 100% | 2 | 98% | 29 |
| 0.50 | 72/114 | 100% | 2 | 97% | 22 |
| 0.55 | 56/114 | 100% | 1 | 98% | 13 |
| 0.60 | 41/114 | 100% | 1 | 98% | 11 |
| 0.65 | 27/114 | 100% | 0 | 100% | 7 |
| 0.70 | 14/114 | 100% | 0 | 100% | 4 |
| 0.75 | 10/114 | 100% | 0 | 100% | 2 |
| 0.80 | 3/114 | 100% | 0 | 100% | 0 |
| 0.85 | 1/114 | 100% | 0 | 100% | 0 |
| 0.90 | 0/114 | - | 0 | - | 0 |
| 0.95 | 0/114 | - | 0 | - | 0 |

Missed comments (5):

- https://github.com/langgenius/dify/pull/35248#discussion_r3087747017 (hint not matched in scope)
- https://github.com/langgenius/dify/pull/44567#discussion_r4585320574 (jev 0.20)
- https://github.com/langgenius/dify/pull/46353#discussion_r4585162272 (jev 0.28)
- https://github.com/langgenius/dify/pull/46458#discussion_r4584131549 (jev 0.28)
- https://github.com/langgenius/dify/pull/46770#discussion_r4656791456 (hint not matched in scope)

Noise: would comment where the pattern is absent (3):

- https://github.com/langgenius/dify/pull/46351#discussion_r4715939583 (judged wrong live)
- https://github.com/langgenius/dify/pull/46463#discussion_r4595823113 (judged wrong live)
- https://github.com/langgenius/dify/pull/46463#discussion_r4609029706 (judged wrong live)

Tolerated: pattern present, reviewer let it through (41):

- https://github.com/langgenius/dify/pull/37950/files api/tests/unit_tests/controllers/openapi/test_contract.py `@@ -5,6 +5,7 @@`
- https://github.com/langgenius/dify/pull/36791/files api/tests/test_containers_integration_tests/controllers/console/workspace/test_members.py `@@ -1,9 +1,14 @@`
- https://github.com/langgenius/dify/pull/40379/files api/tests/unit_tests/fields/test_snippet_fields.py `@@ -1,17 +1,26 @@`
- https://github.com/langgenius/dify/pull/42034/files api/fields/workflow_run_fields.py `@@ -6,15 +6,14 @@`
- https://github.com/langgenius/dify/pull/42335/files api/models/model.py `@@ -1198,12 +1198,7 @@ class Conversation(Base):`
- https://github.com/langgenius/dify/pull/44775/files api/providers/vdb/vdb-baidu/tests/unit_tests/src/dify_vdb_baidu/test_baidu_vector.py `@@ -1,17 +1,21 @@`
- https://github.com/langgenius/dify/pull/44775/files api/providers/vdb/vdb-huawei-cloud/tests/unit_tests/src/dify_vdb_huawei_cloud/test_huawei_cloud_vector.py `@@ -1,7 +1,8 @@`
- https://github.com/langgenius/dify/pull/43112/files api/fields/workflow_draft_variable_fields.py `@@ -3,8 +3,9 @@`
- https://github.com/langgenius/dify/pull/43112/files api/tests/unit_tests/services/test_workflow_draft_variable_service.py `@@ -27,7 +27,7 @@`
- https://github.com/langgenius/dify/pull/44816/files api/tests/unit_tests/core/app/apps/chat/test_generate_response_converter.py `@@ -1,6 +1,8 @@`
- https://github.com/langgenius/dify/pull/44816/files api/tests/unit_tests/core/app/apps/workflow/test_app_config_manager.py `@@ -1,3 +1,4 @@`
- https://github.com/langgenius/dify/pull/44820/files api/tests/unit_tests/core/app/apps/pipeline/test_generate_response_converter.py `@@ -13,7 +13,7 @@`
- https://github.com/langgenius/dify/pull/44822/files api/tests/unit_tests/core/plugin/impl/test_tool.py `@@ -1,3 +1,4 @@`
- https://github.com/langgenius/dify/pull/44822/files api/tests/unit_tests/services/rag_pipeline/pipeline_template/built_in/test_built_in_retrieval.py `@@ -99,7 +99,7 @@ def test_get_builtin_data_reads_from_file_and_caches(mocker: MockerFixture) -> N`
- https://github.com/langgenius/dify/pull/44817/files api/tests/unit_tests/core/app/apps/advanced_chat/test_app_config_manager.py `@@ -1,3 +1,4 @@`
- https://github.com/langgenius/dify/pull/44817/files api/tests/unit_tests/core/plugin/impl/test_agent.py `@@ -1,10 +1,13 @@`
- https://github.com/langgenius/dify/pull/44817/files api/tests/unit_tests/services/test_workflow_restore.py `@@ -5,7 +5,7 @@`
- https://github.com/langgenius/dify/pull/44881/files api/tests/unit_tests/fields/test_dataset_fields.py `@@ -19,13 +19,15 @@`
- https://github.com/langgenius/dify/pull/44881/files api/tests/unit_tests/fields/test_message_fields.py `@@ -3,7 +3,7 @@`
- https://github.com/langgenius/dify/pull/44881/files api/tests/unit_tests/fields/test_workflow_draft_variable_fields.py `@@ -32,7 +32,7 @@`
- https://github.com/langgenius/dify/pull/44875/files api/tests/unit_tests/extensions/otel/decorators/handlers/test_generate_handler.py `@@ -6,11 +6,14 @@`
- https://github.com/langgenius/dify/pull/44885/files api/tests/unit_tests/libs/test_archive_storage.py `@@ -1,10 +1,13 @@`
- https://github.com/langgenius/dify/pull/44885/files api/tests/unit_tests/libs/test_external_api.py `@@ -6,7 +6,8 @@`
- https://github.com/langgenius/dify/pull/44898/files api/providers/vdb/vdb-analyticdb/tests/unit_tests/src/dify_vdb_analyticdb/test_analyticdb_vector_openapi.py `@@ -1,91 +1,121 @@`
- https://github.com/langgenius/dify/pull/45336/files api/tests/unit_tests/core/workflow/nodes/agent_v2/test_session_store.py `@@ -158,7 +158,14 @@ def _install_backend_client(monkeypatch: pytest.MonkeyPatch) -> MagicMock:`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/console/app/test_workflow_run.py `@@ -152,8 +152,8 @@ def test_pause_details_response_schema_is_registered() -> None:`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/inner_api/plugin/test_agent_config.py `@@ -2,8 +2,8 @@`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/data_source/test_website_service.py `@@ -463,7 +463,12 @@ def test_get_crawl_status_typed_rejects_invalid_provider(monkeypatch: pytest.Mon`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/model_provider/test_service.py `@@ -1,4 +1,5 @@`
- https://github.com/langgenius/dify/pull/45725/files api/tests/unit_tests/services/enterprise/test_plugin_manager_service.py `@@ -9,13 +9,13 @@`
- https://github.com/langgenius/dify/pull/45729/files api/tests/unit_tests/core/entities/test_provider_configuration.py `@@ -86,7 +86,7 @@ def _build_provider_configuration(*, provider_name: str = "openai") -> ProviderC`
- https://github.com/langgenius/dify/pull/45726/files api/tests/unit_tests/core/tools/custom_tool/test_tool.py `@@ -10,6 +10,7 @@`
- https://github.com/langgenius/dify/pull/45726/files api/tests/unit_tests/core/tools/mcp_tool/test_provider.py `@@ -43,7 +43,7 @@ def _build_mcp_entity(*, icon: str = "icon.svg") -> MCPProviderEntity:`
- https://github.com/langgenius/dify/pull/45731/files api/tests/unit_tests/core/app/app_config/common/sensitive_word_avoidance/test_manager.py `@@ -20,14 +20,14 @@ class TestSensitiveWordAvoidanceConfigManagerConvert:`
- https://github.com/langgenius/dify/pull/45819/files api/tests/unit_tests/core/workflow/test_human_input_adapter.py `@@ -4,10 +4,12 @@`
- https://github.com/langgenius/dify/pull/45875/files api/tests/unit_tests/core/plugin/impl/test_oauth.py `@@ -1,6 +1,6 @@`
- https://github.com/langgenius/dify/pull/45821/files api/tests/unit_tests/core/app/apps/advanced_chat/test_generate_response_converter.py `@@ -1,4 +1,5 @@`
- https://github.com/langgenius/dify/pull/46404/files api/tests/e2e_tests/contracts/core/workflow/nodes/test_start_node_json_object.py `@@ -1,8 +1,8 @@`
- https://github.com/langgenius/dify/pull/46419/files api/tests/unit_tests/controllers/console/app/test_model_config.py `@@ -3,6 +3,7 @@`
- https://github.com/langgenius/dify/pull/46419/files api/tests/unit_tests/core/plugin/impl/test_trigger.py `@@ -2,8 +2,8 @@`

## precise-types

69 positives (0 outside the rule's paths), 275 negatives, 6 live comments judged wrong; threshold now 0.30, suggested 0.30.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 63/69 | 95% | 5 | 93% | 29 |
| 0.35 | 62/69 | 97% | 5 | 93% | 25 |
| 0.40 | 57/69 | 100% | 5 | 92% | 23 |
| 0.45 | 54/69 | 100% | 5 | 92% | 22 |
| 0.50 | 51/69 | 100% | 5 | 91% | 20 |
| 0.55 | 49/69 | 100% | 5 | 91% | 17 |
| 0.60 | 46/69 | 100% | 4 | 92% | 13 |
| 0.65 | 43/69 | 100% | 3 | 93% | 12 |
| 0.70 | 36/69 | 100% | 3 | 92% | 11 |
| 0.75 | 31/69 | 100% | 2 | 94% | 8 |
| 0.80 | 22/69 | 100% | 2 | 92% | 5 |
| 0.85 | 18/69 | 100% | 2 | 90% | 4 |
| 0.90 | 15/69 | 100% | 1 | 94% | 1 |
| 0.95 | 6/69 | 100% | 1 | 86% | 1 |

Missed comments (6):

- https://github.com/langgenius/dify/pull/36791#discussion_r3321695422 (jev fine 0.40, labelled concrete-type)
- https://github.com/langgenius/dify/pull/43588#discussion_r4196654446 (hint not matched in scope, labelled concrete-type)
- https://github.com/langgenius/dify/pull/46387#discussion_r4585567602 (jev concrete-type 0.23, labelled concrete-type)
- https://github.com/langgenius/dify/pull/46393#discussion_r4584155564 (jev explicit-params 0.25, labelled explicit-params)
- https://github.com/langgenius/dify/pull/46459#discussion_r4583991996 (jev paramspec 0.24, labelled explicit-params)
- https://github.com/langgenius/dify/pull/46461#discussion_r4583990454 (jev concrete-type 0.20, labelled concrete-type)

Noise: would comment where the pattern is absent (5):

- https://github.com/langgenius/dify/pull/45631#discussion_r4610612392 (judged wrong live)
- https://github.com/langgenius/dify/pull/45821#discussion_r4586055716 (judged wrong live)
- https://github.com/langgenius/dify/pull/45970#discussion_r4579152789 (judged wrong live)
- https://github.com/langgenius/dify/pull/46442#discussion_r4692550815 (judged wrong live)
- https://github.com/langgenius/dify/pull/46644#discussion_r4611043750 (judged wrong live)

Tolerated: pattern present, reviewer let it through (29):

- https://github.com/langgenius/dify/pull/37660/files api/core/app/app_config/common/sensitive_word_avoidance/manager.py `@@ -1,10 +1,73 @@`
- https://github.com/langgenius/dify/pull/36791/files api/tests/test_containers_integration_tests/conftest.py `@@ -41,7 +41,7 @@`
- https://github.com/langgenius/dify/pull/39822/files api/controllers/console/flask_admission.py `@@ -0,0 +1,69 @@`
- https://github.com/langgenius/dify/pull/42034/files api/fields/base.py `@@ -1,7 +1,27 @@`
- https://github.com/langgenius/dify/pull/42034/files api/fields/workflow_run_fields.py `@@ -6,15 +6,14 @@`
- https://github.com/langgenius/dify/pull/44775/files api/providers/vdb/vdb-chroma/tests/unit_tests/src/dify_vdb_chroma/test_chroma_vector.py `@@ -2,6 +2,7 @@`
- https://github.com/langgenius/dify/pull/44775/files api/providers/vdb/vdb-huawei-cloud/tests/unit_tests/src/dify_vdb_huawei_cloud/test_huawei_cloud_vector.py `@@ -1,7 +1,8 @@`
- https://github.com/langgenius/dify/pull/43112/files api/fields/workflow_draft_variable_fields.py `@@ -3,8 +3,9 @@`
- https://github.com/langgenius/dify/pull/44458/files api/core/rag/datasource/vdb/vector_integration_test_support.py `@@ -1,4 +1,4 @@`
- https://github.com/langgenius/dify/pull/44458/files api/providers/vdb/vdb-alibabacloud-mysql/tests/unit_tests/test_alibabacloud_mysql_vector.py `@@ -360,8 +360,8 @@ def test_delete_by_ids_table_not_exists(self, mock_pool_class):`
- https://github.com/langgenius/dify/pull/44816/files api/tests/unit_tests/core/app/apps/workflow/test_app_config_manager.py `@@ -1,3 +1,4 @@`
- https://github.com/langgenius/dify/pull/44822/files api/tests/unit_tests/core/plugin/impl/test_tool.py `@@ -1,3 +1,4 @@`
- https://github.com/langgenius/dify/pull/44817/files api/tests/unit_tests/core/app/apps/advanced_chat/test_app_config_manager.py `@@ -1,3 +1,4 @@`
- https://github.com/langgenius/dify/pull/44817/files api/tests/unit_tests/core/plugin/impl/test_agent.py `@@ -1,10 +1,13 @@`
- https://github.com/langgenius/dify/pull/44881/files api/tests/unit_tests/fields/test_dataset_fields.py `@@ -19,13 +19,15 @@`
- https://github.com/langgenius/dify/pull/44881/files api/tests/unit_tests/fields/test_message_fields.py `@@ -3,7 +3,7 @@`
- https://github.com/langgenius/dify/pull/44885/files api/tests/unit_tests/libs/test_archive_storage.py `@@ -1,10 +1,13 @@`
- https://github.com/langgenius/dify/pull/44885/files api/tests/unit_tests/libs/test_external_api.py `@@ -6,7 +6,8 @@`
- https://github.com/langgenius/dify/pull/44898/files api/providers/vdb/vdb-analyticdb/tests/unit_tests/src/dify_vdb_analyticdb/test_analyticdb_vector_openapi.py `@@ -1,91 +1,121 @@`
- https://github.com/langgenius/dify/pull/45336/files api/tests/unit_tests/core/agent/test_base_agent_runner.py `@@ -418,10 +418,10 @@ def test_save_agent_thought_serialization_fallbacks(`
- https://github.com/langgenius/dify/pull/45336/files api/tests/unit_tests/core/workflow/nodes/agent_v2/test_session_store.py `@@ -158,7 +158,14 @@ def _install_backend_client(monkeypatch: pytest.MonkeyPatch) -> MagicMock:`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/console/app/test_workflow_run.py `@@ -152,8 +152,8 @@ def test_pause_details_response_schema_is_registered() -> None:`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/model_provider/test_service.py `@@ -1,4 +1,5 @@`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/test_conversation_service.py `@@ -61,7 +61,7 @@ class ConversationServiceTestDataFactory:`
- https://github.com/langgenius/dify/pull/45725/files api/tests/helpers/unit/services/document_service_validation.py `@@ -106,7 +106,9 @@`
- https://github.com/langgenius/dify/pull/45731/files api/tests/unit_tests/core/agent/test_plugin_entities.py `@@ -59,7 +59,9 @@ class TestAgentStrategyParameterType:`
- https://github.com/langgenius/dify/pull/46188/files api/tests/unit_tests/controllers/web/test_wraps.py `@@ -1,3 +1,4 @@`
- https://github.com/langgenius/dify/pull/45970/files api/core/rag/index_processor/index_processor_base.py `@@ -47,12 +47,12 @@ class BaseIndexProcessor(ABC):`
- https://github.com/langgenius/dify/pull/46191/files api/tests/unit_tests/controllers/service_api/app/test_workflow_events.py `@@ -2,11 +2,11 @@`

## tests-behavior-not-calls

9 positives (0 outside the rule's paths), 207 negatives, 0 live comments judged wrong; threshold now 0.35, suggested 0.35.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 9/9 | 100% | 0 | 100% | 13 |
| 0.35 | 9/9 | 100% | 0 | 100% | 12 |
| 0.40 | 7/9 | 100% | 0 | 100% | 10 |
| 0.45 | 7/9 | 100% | 0 | 100% | 10 |
| 0.50 | 7/9 | 100% | 0 | 100% | 7 |
| 0.55 | 6/9 | 100% | 0 | 100% | 5 |
| 0.60 | 5/9 | 100% | 0 | 100% | 3 |
| 0.65 | 5/9 | 100% | 0 | 100% | 3 |
| 0.70 | 4/9 | 100% | 0 | 100% | 2 |
| 0.75 | 3/9 | 100% | 0 | 100% | 0 |
| 0.80 | 3/9 | 100% | 0 | 100% | 0 |
| 0.85 | 2/9 | 100% | 0 | 100% | 0 |
| 0.90 | 0/9 | - | 0 | - | 0 |
| 0.95 | 0/9 | - | 0 | - | 0 |

Tolerated: pattern present, reviewer let it through (12):

- https://github.com/langgenius/dify/pull/37660/files api/tests/unit_tests/core/app/app_config/common/test_sensitive_word_avoidance_manager.py `@@ -1,6 +1,7 @@`
- https://github.com/langgenius/dify/pull/39822/files api/tests/unit_tests/controllers/console/test_wraps.py `@@ -11,6 +11,7 @@`
- https://github.com/langgenius/dify/pull/39822/files api/tests/unit_tests/controllers/console/workspace/test_workspace.py `@@ -1,8 +1,10 @@`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/controllers/console/datasets/test_datasets_segments.py `@@ -161,7 +161,7 @@ def test_get_success(self, app: Flask):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_dataset.py `@@ -174,18 +174,18 @@ def test_get_datasets_filters_by_maintainer_and_rbac_overrides(self):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_segment.py `@@ -270,9 +270,10 @@ def test_get_child_chunks_applies_keyword_filter_and_paginate(self, account_cont`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -143,8 +143,10 @@ def factory():`
- https://github.com/langgenius/dify/pull/43112/files api/tests/unit_tests/models/test_workflow.py `@@ -36,7 +36,7 @@`
- https://github.com/langgenius/dify/pull/43112/files api/tests/unit_tests/services/test_workflow_draft_variable_service.py `@@ -27,7 +27,7 @@`
- https://github.com/langgenius/dify/pull/44458/files api/providers/vdb/vdb-analyticdb/tests/unit_tests/test_analyticdb_vector_openapi.py `@@ -11,6 +10,7 @@`
- https://github.com/langgenius/dify/pull/45146/files api/tests/unit_tests/services/rag_pipeline/test_rag_pipeline_task_proxy.py `@@ -6,6 +6,7 @@`
- https://github.com/langgenius/dify/pull/45633/files api/tests/unit_tests/commands/test_system.py `@@ -2,6 +2,7 @@`

## tests-real-dependencies

101 positives (0 outside the rule's paths), 207 negatives, 6 live comments judged wrong; threshold now 0.35, suggested 0.35.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 79/101 | 84% | 6 | 93% | 35 |
| 0.35 | 72/101 | 85% | 3 | 96% | 33 |
| 0.40 | 62/101 | 89% | 3 | 95% | 31 |
| 0.45 | 53/101 | 89% | 3 | 95% | 29 |
| 0.50 | 46/101 | 89% | 3 | 94% | 26 |
| 0.55 | 39/101 | 92% | 2 | 95% | 26 |
| 0.60 | 33/101 | 94% | 2 | 94% | 22 |
| 0.65 | 31/101 | 94% | 1 | 97% | 17 |
| 0.70 | 24/101 | 96% | 1 | 96% | 15 |
| 0.75 | 19/101 | 95% | 1 | 95% | 13 |
| 0.80 | 12/101 | 100% | 0 | 100% | 9 |
| 0.85 | 10/101 | 100% | 0 | 100% | 9 |
| 0.90 | 6/101 | 100% | 0 | 100% | 5 |
| 0.95 | 5/101 | 100% | 0 | 100% | 1 |

Missed comments (29):

- https://github.com/langgenius/dify/pull/36191#discussion_r3548937501 (hint not matched in scope, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/36842#discussion_r3338212303 (hint not matched in scope, labelled inject-dependency)
- https://github.com/langgenius/dify/pull/40379#discussion_r3744122730 (jev use-sqlite-session 0.34, labelled import-real-class)
- https://github.com/langgenius/dify/pull/44775#discussion_r4374124655 (hint not matched in scope, labelled import-real-class)
- https://github.com/langgenius/dify/pull/44881#discussion_r4450627568 (jev protocol-di 0.29, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/44888#discussion_r4408644607 (jev use-sqlite-session 0.20, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/44888#discussion_r4420305690 (jev inject-dependency 0.22, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/44888#discussion_r4420307376 (jev use-sqlite-session 0.18, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45320#discussion_r4441505447 (hint not matched in scope, labelled import-real-class)
- https://github.com/langgenius/dify/pull/45331#discussion_r4450195948 (jev use-sqlite-session 0.32, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45466#discussion_r4456386946 (jev inject-dependency 0.26, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45472#discussion_r4464547086 (jev import-real-class 0.27, labelled import-real-class)
- https://github.com/langgenius/dify/pull/45633#discussion_r4488875529 (jev use-sqlite-session 0.24, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45696#discussion_r4491633255 (hint not matched in scope, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45698#discussion_r4489290449 (jev inject-dependency 0.32, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45766#discussion_r4505010078 (hint not matched in scope, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45875#discussion_r4511099640 (hint not matched in scope, labelled import-real-class)
- https://github.com/langgenius/dify/pull/45876#discussion_r4511092860 (jev no-double 0.55, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45896#discussion_r4585314853 (jev inject-dependency 0.32, labelled inject-dependency)
- https://github.com/langgenius/dify/pull/45896#discussion_r4597839789 (jev import-real-class 0.32, labelled import-real-class)
- https://github.com/langgenius/dify/pull/45968#discussion_r4579156703 (jev import-real-class 0.31, labelled import-real-class)
- https://github.com/langgenius/dify/pull/45968#discussion_r4585885225 (jev import-real-class 0.34, labelled import-real-class)
- https://github.com/langgenius/dify/pull/45970#discussion_r4579152789 (hint not matched in scope, labelled import-real-class)
- https://github.com/langgenius/dify/pull/46291#discussion_r4564662547 (jev no-double 0.39, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/46416#discussion_r4585027488 (jev import-real-class 0.19, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/46422#discussion_r4577088037 (hint not matched in scope, labelled import-real-class)
- https://github.com/langgenius/dify/pull/46446#discussion_r4587354672 (hint not matched in scope, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/46450#discussion_r4578596586 (jev inject-dependency 0.29, labelled import-real-class)
- https://github.com/langgenius/dify/pull/46458#discussion_r4583993454 (jev import-real-class 0.29, labelled import-real-class)

Noise: would comment where the pattern is absent (3):

- https://github.com/langgenius/dify/pull/46399#discussion_r4595835666 (judged wrong live)
- https://github.com/langgenius/dify/pull/46453#discussion_r4595828653 (judged wrong live)
- https://github.com/langgenius/dify/pull/46457#discussion_r4692546508 (judged wrong live)

Tolerated: pattern present, reviewer let it through (33):

- https://github.com/langgenius/dify/pull/37639/files api/tests/unit_tests/services/test_knowledge_retrieval_inner_service.py `@@ -74,14 +74,14 @@ def _build_source() -> Source:`
- https://github.com/langgenius/dify/pull/39822/files api/tests/unit_tests/controllers/console/test_wraps.py `@@ -11,6 +11,7 @@`
- https://github.com/langgenius/dify/pull/39822/files api/tests/unit_tests/controllers/console/workspace/test_workspace.py `@@ -1,8 +1,10 @@`
- https://github.com/langgenius/dify/pull/39822/files api/tests/unit_tests/libs/test_login.py `@@ -266,10 +266,10 @@ def test_returns_account_and_tenant_id(self, mocker: MockerFixture):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/controllers/console/app/test_conversation_api.py `@@ -30,7 +30,7 @@ def test_completion_conversation_list_returns_paginated_result(app: Flask, monke`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/controllers/console/workspace/test_workspace.py `@@ -291,7 +291,7 @@ def test_get_success(self, app: Flask):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/controllers/service_api/dataset/test_document.py `@@ -851,9 +851,10 @@ def test_delete_document_dataset_not_found(self, mock_db, mock_doc_svc, app: Fla`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_annotation_service.py `@@ -395,10 +395,11 @@ def test_get_annotation_list_by_app_id_should_return_items_with_keyword(self) ->`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_dataset.py `@@ -174,18 +174,18 @@ def test_get_datasets_filters_by_maintainer_and_rbac_overrides(self):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_segment.py `@@ -270,9 +270,10 @@ def test_get_child_chunks_applies_keyword_filter_and_paginate(self, account_cont`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -143,8 +143,10 @@ def factory():`
- https://github.com/langgenius/dify/pull/41007/files api/tests/unit_tests/controllers/web/test_wraps.py `@@ -1,6 +1,10 @@`
- https://github.com/langgenius/dify/pull/42034/files api/tests/unit_tests/controllers/console/app/test_workflow_run_api.py `@@ -153,13 +153,12 @@ def test_workflow_run_list_returns_frontend_history_contract(`
- https://github.com/langgenius/dify/pull/44775/files api/providers/vdb/vdb-baidu/tests/unit_tests/src/dify_vdb_baidu/test_baidu_vector.py `@@ -1,17 +1,21 @@`
- https://github.com/langgenius/dify/pull/44775/files api/providers/vdb/vdb-tidb-on-qdrant/tests/unit_tests/src/dify_vdb_tidb_on_qdrant/test_tidb_on_qdrant_vector.py `@@ -1,5 +1,4 @@`
- https://github.com/langgenius/dify/pull/43112/files api/tests/unit_tests/controllers/console/app/test_workflow_draft_variable_handlers.py `@@ -0,0 +1,219 @@`
- https://github.com/langgenius/dify/pull/43112/files api/tests/unit_tests/controllers/console/datasets/rag_pipeline/test_rag_pipeline_draft_variable_handlers.py `@@ -0,0 +1,166 @@`
- https://github.com/langgenius/dify/pull/43112/files api/tests/unit_tests/models/test_workflow.py `@@ -36,7 +36,7 @@`
- https://github.com/langgenius/dify/pull/43112/files api/tests/unit_tests/services/test_workflow_draft_variable_service.py `@@ -27,7 +27,7 @@`
- https://github.com/langgenius/dify/pull/44458/files api/providers/vdb/vdb-analyticdb/tests/unit_tests/test_analyticdb_vector_openapi.py `@@ -11,6 +10,7 @@`
- https://github.com/langgenius/dify/pull/44817/files api/tests/unit_tests/core/app/apps/test_message_generator.py `@@ -1,21 +1,39 @@`
- https://github.com/langgenius/dify/pull/44837/files api/tests/unit_tests/core/agent/test_cot_chat_agent_runner.py `@@ -42,17 +42,24 @@ def is_final(self):`
- https://github.com/langgenius/dify/pull/45142/files api/tests/unit_tests/controllers/console/app/test_workflow.py `@@ -12,7 +12,8 @@`
- https://github.com/langgenius/dify/pull/44898/files api/providers/vdb/vdb-analyticdb/tests/unit_tests/src/dify_vdb_analyticdb/test_analyticdb_vector.py `@@ -1,18 +1,75 @@`
- https://github.com/langgenius/dify/pull/44898/files api/providers/vdb/vdb-analyticdb/tests/unit_tests/src/dify_vdb_analyticdb/test_analyticdb_vector_openapi.py `@@ -1,91 +1,121 @@`
- https://github.com/langgenius/dify/pull/45146/files api/tests/unit_tests/services/rag_pipeline/test_rag_pipeline_task_proxy.py `@@ -6,6 +6,7 @@`
- https://github.com/langgenius/dify/pull/45168/files api/tests/unit_tests/core/test_provider_manager.py `@@ -1,3 +1,4 @@`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/model_provider/test_service.py `@@ -1,4 +1,5 @@`
- https://github.com/langgenius/dify/pull/45729/files api/tests/unit_tests/core/entities/test_provider_configuration.py `@@ -86,7 +86,7 @@ def _build_provider_configuration(*, provider_name: str = "openai") -> ProviderC`
- https://github.com/langgenius/dify/pull/45726/files api/tests/unit_tests/core/tools/test_tool_file_manager.py `@@ -12,25 +12,23 @@`
- https://github.com/langgenius/dify/pull/45968/files api/tests/unit_tests/core/prompt/test_advanced_prompt_transform.py `@@ -1,8 +1,8 @@`
- https://github.com/langgenius/dify/pull/46188/files api/tests/unit_tests/controllers/web/test_wraps.py `@@ -1,3 +1,4 @@`
- https://github.com/langgenius/dify/pull/46450/files api/tests/unit_tests/core/tools/builtin_tool/test_tool.py `@@ -1,23 +1,25 @@`

## unrelated-change

7 positives (0 outside the rule's paths), 289 negatives, 0 live comments judged wrong; threshold now 0.90, suggested 0.90.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 4/7 | 100% | 9 | 31% | 0 |
| 0.35 | 4/7 | 100% | 7 | 36% | 0 |
| 0.40 | 4/7 | 100% | 7 | 36% | 0 |
| 0.45 | 4/7 | 100% | 7 | 36% | 0 |
| 0.50 | 4/7 | 100% | 5 | 44% | 0 |
| 0.55 | 4/7 | 100% | 4 | 50% | 0 |
| 0.60 | 4/7 | 100% | 3 | 57% | 0 |
| 0.65 | 4/7 | 100% | 3 | 57% | 0 |
| 0.70 | 4/7 | 100% | 3 | 57% | 0 |
| 0.75 | 4/7 | 100% | 3 | 57% | 0 |
| 0.80 | 4/7 | 100% | 2 | 67% | 0 |
| 0.85 | 4/7 | 100% | 1 | 80% | 0 |
| 0.90 | 4/7 | 100% | 0 | 100% | 0 |
| 0.95 | 2/7 | 100% | 0 | 100% | 0 |

Missed comments (3):

- https://github.com/langgenius/dify/pull/35804#discussion_r3193231742 (jev 0.02)
- https://github.com/langgenius/dify/pull/38280#discussion_r3510172787 (jev 0.05)
- https://github.com/langgenius/dify/pull/43112#discussion_r4536639533 (jev 0.06)

## use-inspect-unwrap

4 positives (0 outside the rule's paths), 275 negatives, 0 live comments judged wrong; threshold now 0.50, suggested 0.50.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 3/4 | 100% | 0 | 100% | 2 |
| 0.35 | 3/4 | 100% | 0 | 100% | 2 |
| 0.40 | 3/4 | 100% | 0 | 100% | 2 |
| 0.45 | 3/4 | 100% | 0 | 100% | 2 |
| 0.50 | 3/4 | 100% | 0 | 100% | 2 |
| 0.55 | 2/4 | 100% | 0 | 100% | 1 |
| 0.60 | 2/4 | 100% | 0 | 100% | 1 |
| 0.65 | 2/4 | 100% | 0 | 100% | 0 |
| 0.70 | 2/4 | 100% | 0 | 100% | 0 |
| 0.75 | 2/4 | 100% | 0 | 100% | 0 |
| 0.80 | 2/4 | 100% | 0 | 100% | 0 |
| 0.85 | 0/4 | - | 0 | - | 0 |
| 0.90 | 0/4 | - | 0 | - | 0 |
| 0.95 | 0/4 | - | 0 | - | 0 |

Missed comments (1):

- https://github.com/langgenius/dify/pull/46362#discussion_r4582179686 (jev 0.28)

Tolerated: pattern present, reviewer let it through (2):

- https://github.com/langgenius/dify/pull/36791/files api/tests/test_containers_integration_tests/controllers/console/workspace/test_members.py `@@ -1,9 +1,14 @@`
- https://github.com/langgenius/dify/pull/46419/files api/tests/unit_tests/controllers/console/app/test_model_config.py `@@ -3,6 +3,7 @@`
