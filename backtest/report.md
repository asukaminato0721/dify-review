# Backtest

1761 hunks. Positives: hunks the reviewer commented on, labelled in `backtest/labels.toml`. Negatives: added hunks in the final diff of the merged pull requests those comments come from. *hits*: positives the rule would have commented on; *fix*: of those, with the labelled option; *noise*: negatives without the rule's hint pattern it would have commented on; *tolerated*: negatives that do contain the pattern (the reviewer let it through at the time) it would have commented on. Precision and the suggestion (max F0.5) count noise only.

## avoid-cast

11 positives (0 outside the rule's paths), 1617 negatives; threshold now 0.95, suggested 0.95.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 11/11 | 100% | 1 | 92% | 3 |
| 0.35 | 11/11 | 100% | 0 | 100% | 3 |
| 0.40 | 11/11 | 100% | 0 | 100% | 3 |
| 0.45 | 11/11 | 100% | 0 | 100% | 3 |
| 0.50 | 11/11 | 100% | 0 | 100% | 3 |
| 0.55 | 11/11 | 100% | 0 | 100% | 3 |
| 0.60 | 11/11 | 100% | 0 | 100% | 3 |
| 0.65 | 11/11 | 100% | 0 | 100% | 3 |
| 0.70 | 11/11 | 100% | 0 | 100% | 3 |
| 0.75 | 11/11 | 100% | 0 | 100% | 3 |
| 0.80 | 11/11 | 100% | 0 | 100% | 3 |
| 0.85 | 11/11 | 100% | 0 | 100% | 3 |
| 0.90 | 11/11 | 100% | 0 | 100% | 3 |
| 0.95 | 11/11 | 100% | 0 | 100% | 3 |

Tolerated: pattern present, reviewer let it through (3):

- https://github.com/langgenius/dify/pull/37950/files api/tests/unit_tests/controllers/openapi/test_contract.py `@@ -100,7 +101,7 @@ def view(*, body):`
- https://github.com/langgenius/dify/pull/36791/files api/tests/test_containers_integration_tests/controllers/console/workspace/test_members.py `@@ -12,24 +17,41 @@`
- https://github.com/langgenius/dify/pull/39822/files api/extensions/ext_application_services.py `@@ -0,0 +1,44 @@`

## inject-session

7 positives (0 outside the rule's paths), 126 negatives; threshold now 0.80, suggested 0.80.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 7/7 | 100% | 2 | 78% | 0 |
| 0.35 | 7/7 | 100% | 2 | 78% | 0 |
| 0.40 | 6/7 | 100% | 0 | 100% | 0 |
| 0.45 | 6/7 | 100% | 0 | 100% | 0 |
| 0.50 | 6/7 | 100% | 0 | 100% | 0 |
| 0.55 | 6/7 | 100% | 0 | 100% | 0 |
| 0.60 | 6/7 | 100% | 0 | 100% | 0 |
| 0.65 | 6/7 | 100% | 0 | 100% | 0 |
| 0.70 | 6/7 | 100% | 0 | 100% | 0 |
| 0.75 | 6/7 | 100% | 0 | 100% | 0 |
| 0.80 | 6/7 | 100% | 0 | 100% | 0 |
| 0.85 | 5/7 | 100% | 0 | 100% | 0 |
| 0.90 | 3/7 | 100% | 0 | 100% | 0 |
| 0.95 | 0/7 | - | 0 | - | 0 |

Missed comments (1):

- https://github.com/langgenius/dify/pull/37639#discussion_r3435612629 (jev 0.36)

## no-bypass-init

2 positives (0 outside the rule's paths), 1617 negatives; threshold now 0.80, suggested 0.80.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 1/2 | 100% | 0 | 100% | 0 |
| 0.35 | 1/2 | 100% | 0 | 100% | 0 |
| 0.40 | 1/2 | 100% | 0 | 100% | 0 |
| 0.45 | 1/2 | 100% | 0 | 100% | 0 |
| 0.50 | 1/2 | 100% | 0 | 100% | 0 |
| 0.55 | 1/2 | 100% | 0 | 100% | 0 |
| 0.60 | 1/2 | 100% | 0 | 100% | 0 |
| 0.65 | 1/2 | 100% | 0 | 100% | 0 |
| 0.70 | 1/2 | 100% | 0 | 100% | 0 |
| 0.75 | 1/2 | 100% | 0 | 100% | 0 |
| 0.80 | 1/2 | 100% | 0 | 100% | 0 |
| 0.85 | 0/2 | - | 0 | - | 0 |
| 0.90 | 0/2 | - | 0 | - | 0 |
| 0.95 | 0/2 | - | 0 | - | 0 |

Missed comments (1):

- https://github.com/langgenius/dify/pull/44775#discussion_r4374124655 (hint not matched in scope)

## no-reflection

10 positives (0 outside the rule's paths), 1617 negatives; threshold now 0.90, suggested 0.90.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 8/10 | 100% | 6 | 57% | 14 |
| 0.35 | 8/10 | 100% | 4 | 67% | 13 |
| 0.40 | 8/10 | 100% | 2 | 80% | 12 |
| 0.45 | 8/10 | 100% | 2 | 80% | 12 |
| 0.50 | 8/10 | 100% | 2 | 80% | 11 |
| 0.55 | 8/10 | 100% | 2 | 80% | 9 |
| 0.60 | 8/10 | 100% | 1 | 89% | 9 |
| 0.65 | 8/10 | 100% | 1 | 89% | 9 |
| 0.70 | 8/10 | 100% | 1 | 89% | 8 |
| 0.75 | 8/10 | 100% | 1 | 89% | 5 |
| 0.80 | 8/10 | 100% | 1 | 89% | 5 |
| 0.85 | 8/10 | 100% | 1 | 89% | 4 |
| 0.90 | 8/10 | 100% | 0 | 100% | 3 |
| 0.95 | 7/10 | 100% | 0 | 100% | 3 |

Missed comments (2):

- https://github.com/langgenius/dify/pull/45458#discussion_r4464598580 (jev 0.07)
- https://github.com/langgenius/dify/pull/45458#discussion_r4464633119 (jev 0.14)

Tolerated: pattern present, reviewer let it through (3):

- https://github.com/langgenius/dify/pull/40379/files api/tests/unit_tests/controllers/console/workspace/test_snippets.py `@@ -39,30 +42,31 @@ def _account(account_id: str = "account-1") -> Account:`
- https://github.com/langgenius/dify/pull/42034/files api/fields/base.py `@@ -1,7 +1,27 @@`
- https://github.com/langgenius/dify/pull/45633/files api/tests/unit_tests/commands/test_system.py `@@ -318,47 +319,60 @@ def test_reset_iterates_all_tenants(monkeypatch: pytest.MonkeyPatch, sqlite_sess`

## no-test-for-type-errors

14 positives (1 outside the rule's paths), 1307 negatives; threshold now 0.85, suggested 0.85.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 11/14 | 100% | 64 | 15% | 0 |
| 0.35 | 11/14 | 100% | 48 | 19% | 0 |
| 0.40 | 10/14 | 100% | 33 | 23% | 0 |
| 0.45 | 10/14 | 100% | 28 | 26% | 0 |
| 0.50 | 10/14 | 100% | 23 | 30% | 0 |
| 0.55 | 10/14 | 100% | 17 | 37% | 0 |
| 0.60 | 10/14 | 100% | 11 | 48% | 0 |
| 0.65 | 9/14 | 100% | 11 | 45% | 0 |
| 0.70 | 7/14 | 100% | 7 | 50% | 0 |
| 0.75 | 6/14 | 100% | 7 | 46% | 0 |
| 0.80 | 6/14 | 100% | 3 | 67% | 0 |
| 0.85 | 6/14 | 100% | 2 | 75% | 0 |
| 0.90 | 4/14 | 100% | 2 | 67% | 0 |
| 0.95 | 2/14 | 100% | 0 | 100% | 0 |

Missed comments (8):

- https://github.com/langgenius/dify/pull/45180#discussion_r4430567006 (jev 0.11)
- https://github.com/langgenius/dify/pull/45180#discussion_r4430567856 (jev 0.11)
- https://github.com/langgenius/dify/pull/45648#discussion_r4488870073 (not asked)
- https://github.com/langgenius/dify/pull/45725#discussion_r4500515284 (jev 0.69)
- https://github.com/langgenius/dify/pull/45726#discussion_r4491694394 (jev 0.73)
- https://github.com/langgenius/dify/pull/45731#discussion_r4491669682 (jev 0.69)
- https://github.com/langgenius/dify/pull/45768#discussion_r4505007988 (jev 0.37)
- https://github.com/langgenius/dify/pull/46404#discussion_r4577205798 (jev 0.61)

Noise: would comment where the pattern is absent (2):

- https://github.com/langgenius/dify/pull/45261/files api/tests/unit_tests/core/rag/models/test_document.py `@@ -0,0 +1,226 @@`
- https://github.com/langgenius/dify/pull/45256/files api/tests/unit_tests/core/rag/splitter/test_text_splitter.py `@@ -2090,3 +2091,117 @@ def test_chunk_count_estimation(self):`

## no-type-ignore

16 positives (0 outside the rule's paths), 1617 negatives; threshold now 0.80, suggested 0.80.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 16/16 | 100% | 0 | 100% | 23 |
| 0.35 | 16/16 | 100% | 0 | 100% | 23 |
| 0.40 | 16/16 | 100% | 0 | 100% | 23 |
| 0.45 | 16/16 | 100% | 0 | 100% | 23 |
| 0.50 | 16/16 | 100% | 0 | 100% | 23 |
| 0.55 | 16/16 | 100% | 0 | 100% | 23 |
| 0.60 | 16/16 | 100% | 0 | 100% | 23 |
| 0.65 | 16/16 | 100% | 0 | 100% | 23 |
| 0.70 | 16/16 | 100% | 0 | 100% | 23 |
| 0.75 | 16/16 | 100% | 0 | 100% | 22 |
| 0.80 | 16/16 | 100% | 0 | 100% | 21 |
| 0.85 | 15/16 | 100% | 0 | 100% | 18 |
| 0.90 | 11/16 | 100% | 0 | 100% | 16 |
| 0.95 | 5/16 | 100% | 0 | 100% | 8 |

Tolerated: pattern present, reviewer let it through (21):

- https://github.com/langgenius/dify/pull/38280/files api/libs/pagination.py `@@ -0,0 +1,86 @@`
- https://github.com/langgenius/dify/pull/43112/files api/tests/unit_tests/controllers/console/app/test_workflow_draft_variable_handlers.py `@@ -0,0 +1,219 @@`
- https://github.com/langgenius/dify/pull/44458/files api/providers/vdb/vdb-alibabacloud-mysql/tests/unit_tests/test_alibabacloud_mysql_vector.py `@@ -645,7 +645,7 @@ def test_unsupported_distance_function(self, mock_pool_class):`
- https://github.com/langgenius/dify/pull/44458/files api/providers/vdb/vdb-clickzetta/tests/unit_tests/test_clickzetta_vector.py `@@ -809,7 +811,7 @@ def test_clickzetta_pool_cleanup_and_shutdown_edge_paths(clickzetta_module):`
- https://github.com/langgenius/dify/pull/44815/files api/tests/unit_tests/libs/test_custom_inputs.py `@@ -8,61 +8,61 @@`
- https://github.com/langgenius/dify/pull/44881/files api/tests/unit_tests/fields/test_dataset_fields.py `@@ -225,7 +234,7 @@ def test_dataset_detail_serializes_after_session_closes(sqlite_session: Session)`
- https://github.com/langgenius/dify/pull/44885/files api/tests/unit_tests/libs/test_archive_storage.py `@@ -144,20 +149,23 @@ def test_init_address_style_override(monkeypatch: pytest.MonkeyPatch, address_st`
- https://github.com/langgenius/dify/pull/44885/files api/tests/unit_tests/libs/test_external_api.py `@@ -78,23 +95,23 @@ def get(self):`
- https://github.com/langgenius/dify/pull/44885/files api/tests/unit_tests/libs/test_external_api.py `@@ -277,12 +294,15 @@ def finalize(self, _e: Exception, data: dict[str, object], _status_code: int) ->`
- https://github.com/langgenius/dify/pull/45320/files api/tests/unit_tests/core/rag/datasource/keyword/jieba/test_jieba_keyword_table_handler.py `@@ -1,140 +1,124 @@`
- https://github.com/langgenius/dify/pull/45336/files api/tests/unit_tests/core/app/app_config/easy_ui_based_app/model_config/test_manager.py `@@ -339,7 +339,7 @@ def test_validate_and_set_defaults_reuses_single_model_assembly():`
- https://github.com/langgenius/dify/pull/45336/files api/tests/unit_tests/core/app/app_config/easy_ui_based_app/model_config/test_manager.py `@@ -365,7 +365,7 @@ def test_convert_keeps_model_config_shape():`
- https://github.com/langgenius/dify/pull/45336/files api/tests/unit_tests/core/workflow/nodes/agent_v2/test_session_store.py `@@ -472,7 +479,7 @@ def test_load_or_create_persists_binding_on_node_execution(`
- https://github.com/langgenius/dify/pull/45336/files api/tests/unit_tests/core/workflow/nodes/agent_v2/test_session_store.py `@@ -564,7 +571,7 @@ def test_load_existing_pointer_reuses_matching_workflow_identity(sqlite_session:`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/console/app/test_workflow_run.py `@@ -217,9 +219,11 @@ def _workflow_run_summary(session: Session, **overrides: object) -> WorkflowRun:`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/console/tag/test_tags.py `@@ -502,10 +498,12 @@ def test_tag_response_normalizes_enum_type(self) -> None:`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/test_conversation_service.py `@@ -61,7 +61,7 @@ class ConversationServiceTestDataFactory:`
- https://github.com/langgenius/dify/pull/45766/files api/tests/unit_tests/core/plugin/impl/test_model_runtime_factory.py `@@ -405,7 +405,7 @@ def test_create_model_type_instance_builds_model_wrappers(`
- https://github.com/langgenius/dify/pull/45766/files api/tests/unit_tests/core/plugin/impl/test_model_runtime_factory.py `@@ -426,13 +426,13 @@ def test_create_model_type_instance_rejects_unsupported_model_type() -> None:`
- https://github.com/langgenius/dify/pull/44875/files api/tests/unit_tests/events/test___init__.py `@@ -3,40 +3,42 @@`
- https://github.com/langgenius/dify/pull/44875/files api/tests/unit_tests/events/test_app_event.py `@@ -190,11 +197,11 @@ def test_requires_caller_session(self, app_model: App) -> None:`

## parse-dont-validate

4 positives (0 outside the rule's paths), 1617 negatives; threshold now 0.90, suggested 0.40.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 2/4 | 100% | 0 | 100% | 64 |
| 0.35 | 2/4 | 100% | 0 | 100% | 48 |
| 0.40 | 2/4 | 100% | 0 | 100% | 42 |
| 0.45 | 1/4 | 100% | 0 | 100% | 37 |
| 0.50 | 1/4 | 100% | 0 | 100% | 27 |
| 0.55 | 1/4 | 100% | 0 | 100% | 21 |
| 0.60 | 1/4 | 100% | 0 | 100% | 13 |
| 0.65 | 0/4 | - | 0 | - | 9 |
| 0.70 | 0/4 | - | 0 | - | 8 |
| 0.75 | 0/4 | - | 0 | - | 4 |
| 0.80 | 0/4 | - | 0 | - | 0 |
| 0.85 | 0/4 | - | 0 | - | 0 |
| 0.90 | 0/4 | - | 0 | - | 0 |
| 0.95 | 0/4 | - | 0 | - | 0 |

Missed comments (2):

- https://github.com/langgenius/dify/pull/35248#discussion_r3087743748 (jev 0.25)
- https://github.com/langgenius/dify/pull/35248#discussion_r3087747017 (hint not matched in scope)

Tolerated: pattern present, reviewer let it through (42):

- https://github.com/langgenius/dify/pull/37950/files api/tests/unit_tests/controllers/openapi/test_contract.py `@@ -100,7 +101,7 @@ def view(*, body):`
- https://github.com/langgenius/dify/pull/42034/files api/fields/workflow_run_fields.py `@@ -185,27 +184,45 @@ def _normalize_timestamp(cls, value: datetime | int | None) -> int | None:`
- https://github.com/langgenius/dify/pull/42335/files api/models/model.py `@@ -1198,12 +1198,7 @@ class Conversation(Base):`
- https://github.com/langgenius/dify/pull/42335/files api/models/model.py `@@ -1519,12 +1513,7 @@ class Message(Base):`
- https://github.com/langgenius/dify/pull/43112/files api/fields/workflow_draft_variable_fields.py `@@ -25,7 +26,27 @@ def _convert_values_to_json_serializable_object(value: Segment) -> RecursiveJSON`
- https://github.com/langgenius/dify/pull/44775/files api/providers/vdb/vdb-baidu/tests/unit_tests/src/dify_vdb_baidu/test_baidu_vector.py `@@ -123,15 +127,154 @@ class FieldType:`
- https://github.com/langgenius/dify/pull/44775/files api/providers/vdb/vdb-baidu/tests/unit_tests/src/dify_vdb_baidu/test_baidu_vector.py `@@ -272,20 +419,25 @@ def test_factory_initializes_collection_name_and_index_struct(baidu_module, monk`
- https://github.com/langgenius/dify/pull/44775/files api/providers/vdb/vdb-baidu/tests/unit_tests/src/dify_vdb_baidu/test_baidu_vector.py `@@ -545,9 +665,14 @@ def test_factory_uses_existing_collection_prefix_when_index_struct_exists(`
- https://github.com/langgenius/dify/pull/44816/files api/tests/unit_tests/core/app/apps/chat/test_generate_response_converter.py `@@ -57,9 +59,12 @@ def stream() -> Generator[ChatbotAppStreamResponse]:`
- https://github.com/langgenius/dify/pull/44817/files api/tests/unit_tests/core/plugin/impl/test_agent.py `@@ -17,13 +20,14 @@ def _agent_provider(name: str = "agent") -> SimpleNamespace:`
- https://github.com/langgenius/dify/pull/44817/files api/tests/unit_tests/core/plugin/impl/test_agent.py `@@ -45,16 +57,19 @@ def fake_request(method, path, type_, **kwargs):`
- https://github.com/langgenius/dify/pull/44822/files api/tests/unit_tests/core/plugin/impl/test_tool.py `@@ -17,14 +18,19 @@ def _tool_provider(name: str = "provider") -> SimpleNamespace:`
- https://github.com/langgenius/dify/pull/44822/files api/tests/unit_tests/core/plugin/impl/test_tool.py `@@ -35,7 +41,13 @@ def fake_request(method, path, type_, **kwargs):`
- https://github.com/langgenius/dify/pull/44822/files api/tests/unit_tests/core/plugin/impl/test_tool.py `@@ -46,20 +58,31 @@ def fake_request(method, path, type_, **kwargs):`
- https://github.com/langgenius/dify/pull/44822/files api/tests/unit_tests/services/rag_pipeline/pipeline_template/built_in/test_built_in_retrieval.py `@@ -99,7 +99,7 @@ def test_get_builtin_data_reads_from_file_and_caches(mocker: MockerFixture) -> N`
- https://github.com/langgenius/dify/pull/44822/files api/tests/unit_tests/services/rag_pipeline/pipeline_template/built_in/test_built_in_retrieval.py `@@ -115,7 +115,7 @@ def test_get_builtin_data_reads_from_file_and_caches(mocker: MockerFixture) -> N`
- https://github.com/langgenius/dify/pull/44820/files api/tests/unit_tests/core/app/apps/pipeline/test_generate_response_converter.py `@@ -35,11 +35,14 @@ def test_convert_blocking_full_and_simple_response():`
- https://github.com/langgenius/dify/pull/44820/files api/tests/unit_tests/core/app/apps/pipeline/test_generate_response_converter.py `@@ -53,11 +56,12 @@ def stream() -> Generator[AppStreamResponse]:`
- https://github.com/langgenius/dify/pull/44820/files api/tests/unit_tests/core/app/apps/pipeline/test_generate_response_converter.py `@@ -105,7 +109,13 @@ def stream() -> Generator[AppStreamResponse]:`
- https://github.com/langgenius/dify/pull/44881/files api/tests/unit_tests/fields/test_dataset_fields.py `@@ -162,11 +164,15 @@ def test_dataset_detail_expands_legacy_null_retrieval_nested_fields():`
- https://github.com/langgenius/dify/pull/44881/files api/tests/unit_tests/fields/test_dataset_fields.py `@@ -193,7 +199,10 @@ def test_dataset_detail_expands_missing_weighted_score_nested_fields():`
- https://github.com/langgenius/dify/pull/44885/files api/tests/unit_tests/libs/test_external_api.py `@@ -424,21 +488,26 @@ def _resolve_by_class(exception: type[Exception]) -> tuple[int, str]:`
- https://github.com/langgenius/dify/pull/44898/files api/providers/vdb/vdb-analyticdb/tests/unit_tests/src/dify_vdb_analyticdb/test_analyticdb_vector_openapi.py `@@ -1,91 +1,121 @@`
- https://github.com/langgenius/dify/pull/44898/files api/providers/vdb/vdb-analyticdb/tests/unit_tests/src/dify_vdb_analyticdb/test_analyticdb_vector_openapi.py `@@ -131,258 +184,193 @@ def test_openapi_config_to_client_params():`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/console/app/test_workflow_run.py `@@ -152,8 +152,8 @@ def test_pause_details_response_schema_is_registered() -> None:`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/inner_api/plugin/test_agent_config.py `@@ -175,14 +171,15 @@ def test_push_happy_path_validates_body_and_preserves_execution_user() -> None:`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/inner_api/plugin/test_agent_config.py `@@ -329,6 +326,7 @@ def test_manifest_invalid_query_returns_400() -> None:`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/inner_api/plugin/test_agent_config.py `@@ -345,6 +343,7 @@ def test_download_request_rejects_invalid_config_name() -> None:`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/inner_api/plugin/test_agent_config.py `@@ -355,6 +354,7 @@ def test_push_invalid_body_returns_400() -> None:`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/data_source/test_website_service.py `@@ -463,7 +463,12 @@ def test_get_crawl_status_typed_rejects_invalid_provider(monkeypatch: pytest.Mon`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/data_source/test_website_service.py `@@ -481,7 +486,12 @@ def test_get_firecrawl_status_adds_time_consuming_when_completed_and_cached(monk`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/data_source/test_website_service.py `@@ -536,8 +546,8 @@ def test_get_jinareader_status_completed_formats_processed_items(monkeypatch: py`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/data_source/test_website_service.py `@@ -687,7 +697,7 @@ def test_get_jinareader_url_data_without_job_id_failure(monkeypatch: pytest.Monk`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/data_source/test_website_service.py `@@ -708,7 +718,7 @@ def test_get_jinareader_url_data_with_job_id_not_completed_raises(monkeypatch: p`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/model_provider/test_service.py `@@ -33,11 +49,64 @@`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/model_provider/test_service.py `@@ -252,27 +356,33 @@ def test_available_credentials_use_injected_query_for_admin_viewer() -> None:`
- https://github.com/langgenius/dify/pull/45726/files api/tests/unit_tests/core/tools/custom_tool/test_tool.py `@@ -409,10 +411,18 @@ def _fake_get(url, **kwargs):`
- https://github.com/langgenius/dify/pull/45726/files api/tests/unit_tests/core/tools/custom_tool/test_tool.py `@@ -421,8 +431,8 @@ def _fake_get(url, **kwargs):`
- https://github.com/langgenius/dify/pull/45726/files api/tests/unit_tests/core/tools/custom_tool/test_tool.py `@@ -454,8 +464,14 @@ def _fake_post(url, **kwargs):`
- https://github.com/langgenius/dify/pull/45731/files api/tests/unit_tests/core/app/app_config/common/sensitive_word_avoidance/test_manager.py `@@ -20,14 +20,14 @@ class TestSensitiveWordAvoidanceConfigManagerConvert:`

## precise-types

11 positives (0 outside the rule's paths), 1617 negatives; threshold now 0.40, suggested 0.35.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 10/11 | 100% | 0 | 100% | 30 |
| 0.35 | 10/11 | 100% | 0 | 100% | 28 |
| 0.40 | 9/11 | 100% | 0 | 100% | 23 |
| 0.45 | 9/11 | 100% | 0 | 100% | 20 |
| 0.50 | 8/11 | 100% | 0 | 100% | 16 |
| 0.55 | 8/11 | 100% | 0 | 100% | 15 |
| 0.60 | 8/11 | 100% | 0 | 100% | 11 |
| 0.65 | 8/11 | 100% | 0 | 100% | 8 |
| 0.70 | 5/11 | 100% | 0 | 100% | 7 |
| 0.75 | 5/11 | 100% | 0 | 100% | 7 |
| 0.80 | 5/11 | 100% | 0 | 100% | 4 |
| 0.85 | 3/11 | 100% | 0 | 100% | 3 |
| 0.90 | 3/11 | 100% | 0 | 100% | 3 |
| 0.95 | 3/11 | 100% | 0 | 100% | 0 |

Missed comments (1):

- https://github.com/langgenius/dify/pull/43588#discussion_r4196654446 (hint not matched in scope, labelled concrete-type)

Tolerated: pattern present, reviewer let it through (28):

- https://github.com/langgenius/dify/pull/37660/files api/core/app/app_config/common/sensitive_word_avoidance/manager.py `@@ -1,10 +1,73 @@`
- https://github.com/langgenius/dify/pull/36791/files api/tests/test_containers_integration_tests/conftest.py `@@ -568,7 +571,7 @@ def mock_ssrf_proxy_requests():`
- https://github.com/langgenius/dify/pull/39822/files api/controllers/console/flask_admission.py `@@ -0,0 +1,69 @@`
- https://github.com/langgenius/dify/pull/40379/files api/tests/unit_tests/controllers/console/workspace/test_snippets.py `@@ -39,30 +42,31 @@ def _account(account_id: str = "account-1") -> Account:`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/controllers/console/app/test_conversation_api.py `@@ -30,7 +30,7 @@ def test_completion_conversation_list_returns_paginated_result(app: Flask, monke`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/controllers/console/app/test_conversation_api.py `@@ -71,7 +71,7 @@ def test_chat_conversation_list_advanced_chat_calls_paginate(app: Flask, monkeyp`
- https://github.com/langgenius/dify/pull/42034/files api/fields/workflow_run_fields.py `@@ -185,27 +184,45 @@ def _normalize_timestamp(cls, value: datetime | int | None) -> int | None:`
- https://github.com/langgenius/dify/pull/43112/files api/fields/workflow_draft_variable_fields.py `@@ -25,7 +26,27 @@ def _convert_values_to_json_serializable_object(value: Segment) -> RecursiveJSON`
- https://github.com/langgenius/dify/pull/44775/files api/providers/vdb/vdb-baidu/tests/unit_tests/src/dify_vdb_baidu/test_baidu_vector.py `@@ -123,15 +127,154 @@ class FieldType:`
- https://github.com/langgenius/dify/pull/44775/files api/providers/vdb/vdb-baidu/tests/unit_tests/src/dify_vdb_baidu/test_baidu_vector.py `@@ -272,20 +419,25 @@ def test_factory_initializes_collection_name_and_index_struct(baidu_module, monk`
- https://github.com/langgenius/dify/pull/44881/files api/tests/unit_tests/fields/test_dataset_fields.py `@@ -19,13 +19,15 @@`
- https://github.com/langgenius/dify/pull/44881/files api/tests/unit_tests/fields/test_dataset_fields.py `@@ -397,7 +406,9 @@ def _count_selects(session: Session) -> Generator[list[str]]:`
- https://github.com/langgenius/dify/pull/44881/files api/tests/unit_tests/fields/test_message_fields.py `@@ -21,67 +21,79 @@ def _retrieved_video_resource():`
- https://github.com/langgenius/dify/pull/44885/files api/tests/unit_tests/libs/test_archive_storage.py `@@ -19,8 +22,8 @@`
- https://github.com/langgenius/dify/pull/44885/files api/tests/unit_tests/libs/test_archive_storage.py `@@ -144,20 +149,23 @@ def test_init_address_style_override(monkeypatch: pytest.MonkeyPatch, address_st`
- https://github.com/langgenius/dify/pull/44898/files api/providers/vdb/vdb-analyticdb/tests/unit_tests/src/dify_vdb_analyticdb/test_analyticdb_vector_openapi.py `@@ -1,91 +1,121 @@`
- https://github.com/langgenius/dify/pull/45142/files api/tests/unit_tests/controllers/console/app/test_workflow.py `@@ -228,6 +228,41 @@ def test_publish_workflow_returns_success(`
- https://github.com/langgenius/dify/pull/45336/files api/tests/unit_tests/core/workflow/nodes/agent_v2/test_session_store.py `@@ -158,7 +158,14 @@ def _install_backend_client(monkeypatch: pytest.MonkeyPatch) -> MagicMock:`
- https://github.com/langgenius/dify/pull/45458/files api/tests/unit_tests/controllers/console/app/test_workflow_run.py `@@ -152,8 +152,8 @@ def test_pause_details_response_schema_is_registered() -> None:`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/test_conversation_service.py `@@ -61,7 +61,7 @@ class ConversationServiceTestDataFactory:`
- https://github.com/langgenius/dify/pull/45472/files api/tests/unit_tests/services/test_conversation_service.py `@@ -79,39 +79,44 @@ def create_account(account_id: str = ACCOUNT_ID, **kwargs) -> Account:`
- https://github.com/langgenius/dify/pull/45698/files api/tests/unit_tests/services/knowledge/summaries/test_adapters.py `@@ -196,7 +196,7 @@ def test_generate_uses_document_language(self, sqlite_session: Session, monkeypa`
- https://github.com/langgenius/dify/pull/45698/files api/tests/unit_tests/services/knowledge/summaries/test_adapters.py `@@ -304,7 +304,7 @@ def test_provided_session_releases_transaction_before_every_vector_attempt(`
- https://github.com/langgenius/dify/pull/45726/files api/tests/unit_tests/core/tools/custom_tool/test_tool.py `@@ -393,9 +395,9 @@ def test_do_http_request_builds_arguments_and_handles_invalid_method(monkeypatch`
- https://github.com/langgenius/dify/pull/45725/files api/tests/helpers/unit/services/document_service_validation.py `@@ -155,7 +157,7 @@ def create_dataset_mock(`
- https://github.com/langgenius/dify/pull/45725/files api/tests/helpers/unit/services/document_service_validation.py `@@ -188,7 +190,7 @@ def create_knowledge_config(`
- https://github.com/langgenius/dify/pull/46188/files api/tests/unit_tests/controllers/web/test_wraps.py `@@ -575,15 +584,17 @@ def post(self, _app: _wraps_App, _end_user: _wraps_EndUser) -> Never:`
- https://github.com/langgenius/dify/pull/46191/files api/tests/unit_tests/controllers/service_api/app/test_workflow_events.py `@@ -77,11 +80,13 @@ def enter(self, request_id: str | None = None) -> str:`

## tests-behavior-not-calls

0 positives (0 outside the rule's paths), 1405 negatives; threshold now 0.70, suggested none (no threshold reaches 50% precision).

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 0/0 | - | 0 | - | 26 |
| 0.35 | 0/0 | - | 0 | - | 26 |
| 0.40 | 0/0 | - | 0 | - | 25 |
| 0.45 | 0/0 | - | 0 | - | 24 |
| 0.50 | 0/0 | - | 0 | - | 22 |
| 0.55 | 0/0 | - | 0 | - | 21 |
| 0.60 | 0/0 | - | 0 | - | 20 |
| 0.65 | 0/0 | - | 0 | - | 19 |
| 0.70 | 0/0 | - | 0 | - | 14 |
| 0.75 | 0/0 | - | 0 | - | 12 |
| 0.80 | 0/0 | - | 0 | - | 5 |
| 0.85 | 0/0 | - | 0 | - | 3 |
| 0.90 | 0/0 | - | 0 | - | 1 |
| 0.95 | 0/0 | - | 0 | - | 0 |

Tolerated: pattern present, reviewer let it through (14):

- https://github.com/langgenius/dify/pull/37660/files api/tests/unit_tests/core/app/app_config/common/test_sensitive_word_avoidance_manager.py `@@ -156,7 +168,7 @@ def test_validate_calls_moderation_factory(self, mocker: MockerFixture):`
- https://github.com/langgenius/dify/pull/37660/files api/tests/unit_tests/core/app/app_config/common/test_sensitive_word_avoidance_manager.py `@@ -178,7 +190,7 @@ def test_validate_sets_empty_dict_when_config_none(self, mocker: MockerFixture):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/controllers/console/datasets/test_datasets_segments.py `@@ -1267,7 +1267,7 @@ def test_segment_list_postgres_keyword_filter_handles_scalar_keywords(self, app:`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_dataset.py `@@ -198,26 +198,26 @@ def test_get_datasets_filters_by_maintainer_and_rbac_overrides(self):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_dataset.py `@@ -286,39 +286,39 @@ def test_get_datasets_rbac_include_all_uses_workspace_permission(self):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_dataset.py `@@ -329,8 +329,8 @@ def test_get_datasets_legacy_owner_include_all_keeps_full_access(self):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_segment.py `@@ -285,7 +286,7 @@ def test_get_child_chunks_applies_keyword_filter_and_paginate(self, account_cont`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_segment.py `@@ -340,7 +342,7 @@ def test_get_segments_uses_status_and_keyword_filters(self):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -168,11 +170,11 @@ def test_get_external_knowledge_apis_success_basic(self, mock_db, factory: Exter`
- https://github.com/langgenius/dify/pull/44458/files api/providers/vdb/vdb-analyticdb/tests/unit_tests/test_analyticdb_vector_openapi.py `@@ -181,7 +184,7 @@ def test_initialize_runs_when_cache_is_missing(monkeypatch: pytest.MonkeyPatch):`
- https://github.com/langgenius/dify/pull/44458/files api/providers/vdb/vdb-analyticdb/tests/unit_tests/test_analyticdb_vector_openapi.py `@@ -252,7 +256,7 @@ def test_create_collection_if_not_exists_creates_when_missing(monkeypatch: pytes`
- https://github.com/langgenius/dify/pull/45146/files api/tests/unit_tests/services/rag_pipeline/test_rag_pipeline_task_proxy.py `@@ -332,7 +321,7 @@ def test_upload_invoke_entities(self, mock_db: MagicMock, mock_file_service_clas`
- https://github.com/langgenius/dify/pull/45633/files api/tests/unit_tests/commands/test_system.py `@@ -369,18 +383,20 @@ def _upgrade():`
- https://github.com/langgenius/dify/pull/45633/files api/tests/unit_tests/commands/test_system.py `@@ -390,12 +406,14 @@ def test_upgrade_db_success_ignores_lock_not_owned_on_release(monkeypatch, capsy`

## tests-real-dependencies

36 positives (0 outside the rule's paths), 1405 negatives; threshold now 0.30, suggested 0.30.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 20/36 | 80% | 0 | 100% | 105 |
| 0.35 | 19/36 | 79% | 0 | 100% | 97 |
| 0.40 | 17/36 | 82% | 0 | 100% | 84 |
| 0.45 | 17/36 | 82% | 0 | 100% | 77 |
| 0.50 | 17/36 | 82% | 0 | 100% | 65 |
| 0.55 | 15/36 | 87% | 0 | 100% | 59 |
| 0.60 | 14/36 | 93% | 0 | 100% | 51 |
| 0.65 | 12/36 | 92% | 0 | 100% | 47 |
| 0.70 | 9/36 | 89% | 0 | 100% | 35 |
| 0.75 | 8/36 | 100% | 0 | 100% | 27 |
| 0.80 | 8/36 | 100% | 0 | 100% | 19 |
| 0.85 | 7/36 | 100% | 0 | 100% | 12 |
| 0.90 | 7/36 | 100% | 0 | 100% | 8 |
| 0.95 | 5/36 | 100% | 0 | 100% | 4 |

Missed comments (16):

- https://github.com/langgenius/dify/pull/36191#discussion_r3548937501 (hint not matched in scope, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/36842#discussion_r3338212303 (hint not matched in scope, labelled inject-dependency)
- https://github.com/langgenius/dify/pull/44775#discussion_r4374124655 (hint not matched in scope, labelled import-real-class)
- https://github.com/langgenius/dify/pull/44888#discussion_r4408644607 (jev use-sqlite-session 0.26, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/44888#discussion_r4420305690 (jev no-double 0.85, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/44888#discussion_r4420307376 (jev use-sqlite-session 0.26, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45320#discussion_r4441505447 (hint not matched in scope, labelled import-real-class)
- https://github.com/langgenius/dify/pull/45458#discussion_r4464586759 (hint not matched in scope, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45633#discussion_r4488875529 (jev no-double 0.42, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45696#discussion_r4491633255 (hint not matched in scope, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45698#discussion_r4489290449 (jev no-double 0.84, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45766#discussion_r4505010363 (hint not matched in scope, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/45875#discussion_r4511099640 (hint not matched in scope, labelled import-real-class)
- https://github.com/langgenius/dify/pull/45876#discussion_r4511092860 (jev no-double 0.90, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/46291#discussion_r4564662547 (jev no-double 0.49, labelled use-sqlite-session)
- https://github.com/langgenius/dify/pull/46422#discussion_r4577088037 (hint not matched in scope, labelled import-real-class)

Tolerated: pattern present, reviewer let it through (105):

- https://github.com/langgenius/dify/pull/37639/files api/tests/unit_tests/services/test_knowledge_retrieval_inner_service.py `@@ -74,14 +74,14 @@ def _build_source() -> Source:`
- https://github.com/langgenius/dify/pull/37639/files api/tests/unit_tests/services/test_knowledge_retrieval_inner_service.py `@@ -151,8 +150,9 @@ def test_retrieve_maps_single_request(self, mock_db, mock_rag_cls):`
- https://github.com/langgenius/dify/pull/37639/files api/tests/unit_tests/services/test_knowledge_retrieval_inner_service.py `@@ -184,35 +184,35 @@ def test_retrieve_maps_single_request(self, mock_db, mock_rag_cls):`
- https://github.com/langgenius/dify/pull/39822/files api/tests/unit_tests/controllers/console/test_wraps.py `@@ -124,6 +127,45 @@ def protected_view():`
- https://github.com/langgenius/dify/pull/39822/files api/tests/unit_tests/controllers/console/workspace/test_workspace.py `@@ -35,21 +37,57 @@`
- https://github.com/langgenius/dify/pull/39822/files api/tests/unit_tests/controllers/console/workspace/test_workspace.py `@@ -71,201 +109,198 @@ def make_tenant(`
- https://github.com/langgenius/dify/pull/39822/files api/tests/unit_tests/libs/test_login.py `@@ -334,7 +334,11 @@ def test_falls_back_to_current_account_tenant(self, mocker: MockerFixture) -> No`
- https://github.com/langgenius/dify/pull/40379/files api/tests/unit_tests/controllers/console/workspace/test_snippets.py `@@ -213,40 +219,64 @@ def test_create_snippet_rejects_forbidden_nodes(app: Flask, monkeypatch: pytest.`
- https://github.com/langgenius/dify/pull/40379/files api/tests/unit_tests/controllers/console/workspace/test_snippets.py `@@ -261,27 +291,20 @@ def test_patch_snippet_returns_400_for_empty_payload(app: Flask, monkeypatch: py`
- https://github.com/langgenius/dify/pull/40379/files api/tests/unit_tests/controllers/console/workspace/test_snippets.py `@@ -293,17 +316,125 @@ def __init__(self, engine, *args, **kwargs):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/controllers/console/app/test_conversation_api.py `@@ -30,7 +30,7 @@ def test_completion_conversation_list_returns_paginated_result(app: Flask, monke`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/controllers/console/app/test_conversation_api.py `@@ -71,7 +71,7 @@ def test_chat_conversation_list_advanced_chat_calls_paginate(app: Flask, monkeyp`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/controllers/console/workspace/test_workspace.py `@@ -291,7 +291,7 @@ def test_get_success(self, app: Flask):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/controllers/console/workspace/test_workspace.py `@@ -308,7 +308,7 @@ def test_get_has_next_true(self, app: Flask):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/controllers/service_api/dataset/test_document.py `@@ -851,9 +851,10 @@ def test_delete_document_dataset_not_found(self, mock_db, mock_doc_svc, app: Fla`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_annotation_service.py `@@ -395,10 +395,11 @@ def test_get_annotation_list_by_app_id_should_return_items_with_keyword(self) ->`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_annotation_service.py `@@ -417,9 +418,10 @@ def test_get_annotation_list_by_app_id_should_return_items_without_keyword(self)`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_annotation_service.py `@@ -1101,9 +1103,11 @@ def test_get_annotation_hit_histories_should_return_items_and_total(self) -> Non`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_dataset.py `@@ -174,18 +174,18 @@ def test_get_datasets_filters_by_maintainer_and_rbac_overrides(self):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_dataset.py `@@ -198,26 +198,26 @@ def test_get_datasets_filters_by_maintainer_and_rbac_overrides(self):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_dataset.py `@@ -227,21 +227,21 @@ def test_get_datasets_filters_only_by_rbac_overrides_without_manage_own_permissi`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_dataset.py `@@ -262,20 +262,20 @@ def test_get_datasets_by_ids_applies_rbac_visibility(self):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_dataset.py `@@ -286,39 +286,39 @@ def test_get_datasets_rbac_include_all_uses_workspace_permission(self):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_segment.py `@@ -270,9 +270,10 @@ def test_get_child_chunks_applies_keyword_filter_and_paginate(self, account_cont`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_dataset_service_segment.py `@@ -324,9 +325,10 @@ def test_get_segments_uses_status_and_keyword_filters(self):`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -143,8 +143,10 @@ def factory():`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -168,11 +170,11 @@ def test_get_external_knowledge_apis_success_basic(self, mock_db, factory: Exter`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -196,14 +198,16 @@ def test_get_external_knowledge_apis_with_search_filter(`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -214,9 +218,9 @@ def test_get_external_knowledge_apis_empty_results(self, mock_db, factory: Exter`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -236,9 +240,9 @@ def test_get_external_knowledge_apis_large_result_set(`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -258,9 +262,9 @@ def test_get_external_knowledge_apis_pagination_last_page(`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -283,9 +287,9 @@ def test_get_external_knowledge_apis_case_insensitive_search(`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -304,9 +308,9 @@ def test_get_external_knowledge_apis_special_characters_search(`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -315,20 +319,20 @@ def test_get_external_knowledge_apis_max_per_page_limit(`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -433,13 +437,13 @@ def test_validate_api_list_with_extra_fields(self, factory: ExternalDatasetServi`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -458,57 +462,55 @@ def test_create_external_knowledge_api_success_full(`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -517,40 +519,40 @@ def test_create_external_knowledge_api_settings_json_serialization(`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -822,43 +824,43 @@ def test_check_endpoint_authorization_header_format(`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -873,24 +875,24 @@ def test_update_external_knowledge_api_success_all_fields(`
- https://github.com/langgenius/dify/pull/38280/files api/tests/unit_tests/services/test_external_dataset_service.py `@@ -905,63 +907,59 @@ def test_update_external_knowledge_api_preserve_hidden_api_key(`

## unrelated-change

6 positives (0 outside the rule's paths), 1645 negatives; threshold now 0.90, suggested 0.90.

| threshold | hits | fix | noise | precision | tolerated |
|---|---|---|---|---|---|
| 0.30 | 4/6 | 100% | 46 | 8% | 0 |
| 0.35 | 4/6 | 100% | 37 | 10% | 0 |
| 0.40 | 4/6 | 100% | 31 | 11% | 0 |
| 0.45 | 4/6 | 100% | 21 | 16% | 0 |
| 0.50 | 4/6 | 100% | 16 | 20% | 0 |
| 0.55 | 4/6 | 100% | 9 | 31% | 0 |
| 0.60 | 4/6 | 100% | 5 | 44% | 0 |
| 0.65 | 4/6 | 100% | 3 | 57% | 0 |
| 0.70 | 4/6 | 100% | 3 | 57% | 0 |
| 0.75 | 4/6 | 100% | 3 | 57% | 0 |
| 0.80 | 4/6 | 100% | 2 | 67% | 0 |
| 0.85 | 2/6 | 100% | 1 | 67% | 0 |
| 0.90 | 2/6 | 100% | 0 | 100% | 0 |
| 0.95 | 1/6 | 100% | 0 | 100% | 0 |

Missed comments (4):

- https://github.com/langgenius/dify/pull/35804#discussion_r3193231742 (jev 0.02)
- https://github.com/langgenius/dify/pull/38280#discussion_r3510172787 (jev 0.05)
- https://github.com/langgenius/dify/pull/43112#discussion_r4536639533 (jev 0.84)
- https://github.com/langgenius/dify/pull/43112#discussion_r4536640405 (jev 0.82)
