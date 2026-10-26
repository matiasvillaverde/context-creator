# Open-source language audit — 2026-10-26

Feature support is uneven. This audit found concrete failures on real projects;
parser registration and passing fixture tests provide limited evidence about
per-language feature coverage.

The audited product source is `e2e36f51382e18c2810ca187b4c105ea3a165c64` (after PR #90).
This change records the audit and its tooling. The product failures below remain
open; this report does not claim they were fixed.

## Method

- Selected one widely used public project for each of the 29 registered families.
  Repository identity, commit, license metadata and star count are pinned in
  [the manifest](../benchmarks/oss-corpus.json).
- Selected three files per family once, using random seed
  `38b6c7f095ce690ac6b7bea86a6ac394`. Every selected file was retained.
- Eligibility: ordinary, non-hidden UTF-8 source, at most 64 KiB and 600 lines;
  standard dependency/build directories excluded. Tests and negative fixtures
  remained eligible. The sample contains 87 files and 12,271 source lines.
- Manually inspected source declarations/imports and representative type/call/body
  regions in every file. Small files were read in full. Large files were reviewed
  through declaration/import windows and representative bodies. This is a focused
  semantic audit, not a line-by-line review of all upstream application logic.
- Recorded independent source expectations and review notes for all 87 files in
  [the manual checks](../benchmarks/oss-oracles.json), then compared actual output.
- Ran repository maps on all 87 files, import/caller/type CLI expansion on one
  file per family, and crate analysis/context/source-read checks on the sample.
  Supplemental checks isolated declaration extraction, repeated timed-out commands
  alone, and exercised revision-checked sessions.
- Used pinned source archives in an external cache. No upstream application,
  build script, database query or test suite was executed. Public source was
  scanned with secret filtering off so redaction would not confound spans.

## Observed results

| Check | Result |
| --- | --- |
| Repository-map commands | 86/87 exited successfully; successful exit still hid missing declarations |
| Combined import/caller/type expansion | 26/29 completed within the initial 90-second limit |
| Full semantic/crate cases | 81/87 completed; Java and Kotlin project probes timed out at 420 seconds |
| Supplemental declaration-only cases | All six Java/Kotlin files completed; Kotlin's slow map input parsed in 15.5 ms |
| Complete context JSON token budgets | 81/81 completed cases had exact counts within 8,192 tokens |
| Full snippet/source comparisons | All 1,826 full snippets matched source within their reported line ranges |
| Source-page checks | 78/81 initially; the three SQL reads passed with neutral global Git configuration |
| Session revision/edit checks | 27 families completed; Java/Kotlin exceeded the supplemental 180-second limit |

All 27 completed session checks reused unchanged analysis, detected the edit and
rejected a stale revision. Unchanged requests can deliver previously omitted
snippets. Three explicitly reset their context and correctly supplied complete
replacements; the others did not resend previously delivered content.

The initial run used three concurrent project audits. Standalone confirmation
kept the Kotlin map, Kotlin expansion, and Java expansion failures at 90 seconds.
Swift expansion completed alone in 51.4 seconds, so its initial timeout is a load
sensitivity observation. Timings describe this host and these commands; they are
not portable performance guarantees.

The [machine-readable results](../benchmarks/results/2026-10-26-oss-audit.json)
retain per-file expected/observed facts, command outcomes, source hashes and
upstream links. Missing-expectation counts cover only manually chosen checks;
they are not exhaustive recall scores. The sample exercises no DDL definitions
in its SQL files, for example.

## Concrete failures and limitations

1. **JS/TS parent imports:** Zod's `../core/checks.js` and Express's
   `../../lib/express` were rejected as invalid module names even though the
   targets are inside the repository. Relative-path validation rejects `..`
   before confinement-aware file resolution.
2. **Dropped imports:** Scala imports such as `cats.data.ContT` became `cats`.
   Julia's plain `using BenchmarkTools`, `using Flux`, and `using CUDA` were
   omitted. A Zod `import { z }` was omitted. Haskell additionally emitted a
   computed-import placeholder for ordinary import keyword tokens.
3. **Missing map declarations:** Express prototype/object functions, nested Dart
   timeout helpers, Java annotation `@interface`, Kotlin named objects, Clojure
   `defmethod`, Haskell point-free functions, and a self-closing HTML image ID
   were absent from their extracted symbol sets. Whole-file/context fallback
   often retained the source, but those forms lacked reliable map navigation.
4. **Partial calls and types:** Haskell/Clojure have no call-query extraction;
   SQL has no function-call extraction. TypeScript's sampled annotations produced
   no type references, chained `safeParse` calls were missing, and Dart generic
   `_transform` calls were missed. Go composite-literal and PHP aliased-parent
   type references were also absent.
5. **Visibility metadata:** Profile analyzers marked clear local/private functions
   as exported, including Dart `_transform`, Elixir `defp now`, and Lua local
   functions. These flags do not establish public API visibility.
6. **Scaling:** Kotlin's map did not finish within 90 seconds even though direct
   declaration extraction on that same file completed in milliseconds. Java and
   Kotlin full-root analysis also exceeded the 420-second project allowance.
7. **Workspace/build binding:** Zod path aliases and Dio workspace `package:`
   imports were treated as external. Namespace/search-path guessing cannot
   establish compiler-level bindings in multi-module repositories.
8. **Dialect/preprocessor limits:** A macro-wrapped fmt header reported a
   recoverable parse gap. The PostgreSQL sparse-vector expressions and psql client
   commands also reported gaps. The malformed Mojolicious fixture is intentional;
   its diagnostic is expected. A parser diagnostic does not establish that an
   upstream project is compiler-invalid.
9. **Ignore-policy consistency:** The initial host's global `*.sql` ignore blocked
   `read_file` while archive context ingestion included those files. Child-only
   neutral Git configuration made the same three page reads pass, without
   changing user configuration or redrawing samples. Archive walking and source
   reads apply global ignores differently.

An empty `relations` or `gaps` array is not proof of complete resolution: many
8,192-token responses reported substantial `truncated_metadata` and snippet
omissions. The audit uses direct resolver observations to identify resolution
failures and does not classify omitted metadata as a missing graph edge.

## Per-language manual review

Each row links the three immutable sampled files. A row with recovered forms
only describes the expectations checked in those files.

| Family | Project | Random samples | Manual observations |
| --- | --- | --- | --- |
| Rust | [tokio-rs/tokio](https://github.com/tokio-rs/tokio) | [1: task_local.rs](https://github.com/tokio-rs/tokio/blob/6e3d753fccf81e91567f6bc8799fdc1df860bc47/tokio/tests/task_local.rs), [2: mod.rs](https://github.com/tokio-rs/tokio/blob/6e3d753fccf81e91567f6bc8799fdc1df860bc47/tokio-util/src/sync/tests/mod.rs), [3: dump.rs](https://github.com/tokio-rs/tokio/blob/6e3d753fccf81e91567f6bc8799fdc1df860bc47/tokio/src/runtime/dump.rs) | Checked declarations, calls, types and conditional module import recovered; cfg/macro expansion unverified. |
| Python | [pallets/flask](https://github.com/pallets/flask) | [1: templating.py](https://github.com/pallets/flask/blob/d086db856be187255b8ec61ef409357393020f32/src/flask/templating.py), [2: ctx.py](https://github.com/pallets/flask/blob/d086db856be187255b8ec61ef409357393020f32/src/flask/ctx.py), [3: test_config.py](https://github.com/pallets/flask/blob/d086db856be187255b8ec61ef409357393020f32/tests/test_config.py) | Checked nested functions, annotations and relative imports recovered; runtime loaders remain unverified. |
| JavaScript | [expressjs/express](https://github.com/expressjs/express) | [1: request.js](https://github.com/expressjs/express/blob/1c6f1b0fe4000977b9a7b4f34097e4d527dd9824/lib/request.js), [2: index.js](https://github.com/expressjs/express/blob/1c6f1b0fe4000977b9a7b4f34097e4d527dd9824/examples/view-locals/index.js), [3: index.js](https://github.com/expressjs/express/blob/1c6f1b0fe4000977b9a7b4f34097e4d527dd9824/examples/route-map/index.js) | Assigned/prototype functions missing from maps; valid parent-relative imports rejected. |
| TypeScript | [colinhacks/zod](https://github.com/colinhacks/zod) | [1: mk.ts](https://github.com/colinhacks/zod/blob/afdac65e0f80c0ccf9e8909da97c411567908b57/packages/zod/src/v4/locales/mk.ts), [2: page.tsx](https://github.com/colinhacks/zod/blob/afdac65e0f80c0ccf9e8909da97c411567908b57/packages/docs/app/blog/%5Bslug%5D/page.tsx), [3: fr.test.ts](https://github.com/colinhacks/zod/blob/afdac65e0f80c0ccf9e8909da97c411567908b57/packages/zod/src/v4/core/tests/locales/fr.test.ts) | Type references and chained calls missing; one import omitted; parent paths rejected and aliases unresolved. |
| Go | [gin-gonic/gin](https://github.com/gin-gonic/gin) | [1: errors_test.go](https://github.com/gin-gonic/gin/blob/0f09c3a9b4626d9fe9979cf4d0521d6ae32ab646/errors_test.go), [2: gins.go](https://github.com/gin-gonic/gin/blob/0f09c3a9b4626d9fe9979cf4d0521d6ae32ab646/ginS/gins.go), [3: uri.go](https://github.com/gin-gonic/gin/blob/0f09c3a9b4626d9fe9979cf4d0521d6ae32ab646/binding/uri.go) | Composite-literal Error type reference missing; implicit package context is not an explicit import edge. |
| Swift | [Alamofire/Alamofire](https://github.com/Alamofire/Alamofire) | [1: HTTPMethod.swift](https://github.com/Alamofire/Alamofire/blob/bda9ed57d72988a3a2ada33d824583541f86eac6/Source/Core/HTTPMethod.swift), [2: RetryPolicyTests.swift](https://github.com/Alamofire/Alamofire/blob/bda9ed57d72988a3a2ada33d824583541f86eac6/Tests/RetryPolicyTests.swift), [3: AlamofireExtended.swift](https://github.com/Alamofire/Alamofire/blob/bda9ed57d72988a3a2ada33d824583541f86eac6/Source/Features/AlamofireExtended.swift) | Checked forms recovered; expansion timed out concurrently, then completed in 51.4 seconds alone. |
| Java | [junit-team/junit-framework](https://github.com/junit-team/junit-framework) | [1: JUnit4TestCaseWithNotFilterableRunner.java](https://github.com/junit-team/junit-framework/blob/bf8b0f4821e5445d5d2c0f1296d523853f1b6ddb/junit-vintage-engine/src/testFixtures/java/org/junit/vintage/engine/samples/junit4/JUnit4TestCaseWithNotFilterableRunner.java), [2: AssertEquals.java](https://github.com/junit-team/junit-framework/blob/bf8b0f4821e5445d5d2c0f1296d523853f1b6ddb/junit-jupiter-api/src/main/java/org/junit/jupiter/api/AssertEquals.java), [3: DisabledInNativeImage.java](https://github.com/junit-team/junit-framework/blob/bf8b0f4821e5445d5d2c0f1296d523853f1b6ddb/junit-jupiter-api/src/main/java/org/junit/jupiter/api/condition/DisabledInNativeImage.java) | Annotation declaration missing; combined expansion and full analysis hit time limits. |
| C | [redis/redis](https://github.com/redis/redis) | [1: lolwut6.c](https://github.com/redis/redis/blob/702897679328fef1a350945bafb26b570277880c/src/lolwut6.c), [2: infotest.c](https://github.com/redis/redis/blob/702897679328fef1a350945bafb26b570277880c/tests/modules/infotest.c), [3: dict.c](https://github.com/redis/redis/blob/702897679328fef1a350945bafb26b570277880c/deps/hiredis/dict.c) | Checked functions, structs and includes recovered; macro/callback binding remains approximate. |
| C++ | [fmtlib/fmt](https://github.com/fmtlib/fmt) | [1: gtest-extra.cc](https://github.com/fmtlib/fmt/blob/f87a75e15262614ce8aa0e6a0664df9d0823fb92/test/gtest-extra.cc), [2: assert-test.cc](https://github.com/fmtlib/fmt/blob/f87a75e15262614ce8aa0e6a0664df9d0823fb92/test/assert-test.cc), [3: ostream.h](https://github.com/fmtlib/fmt/blob/f87a75e15262614ce8aa0e6a0664df9d0823fb92/include/fmt/ostream.h) | Template/macro header produced a parse gap and an omitted formatter type. |
| C# | [serilog/serilog](https://github.com/serilog/serilog) | [1: GlobalUsings.cs](https://github.com/serilog/serilog/blob/43105b35095337d77f022561a8f2d3db2f9a7937/test/Serilog.PerformanceTests/GlobalUsings.cs), [2: ReusableStringWriter.cs](https://github.com/serilog/serilog/blob/43105b35095337d77f022561a8f2d3db2f9a7937/src/Serilog/Rendering/ReusableStringWriter.cs), [3: AllocationsBenchmark.cs](https://github.com/serilog/serilog/blob/43105b35095337d77f022561a8f2d3db2f9a7937/test/Serilog.PerformanceTests/AllocationsBenchmark.cs) | Checked declarations/calls/types recovered; global/implicit namespace binding remains approximate. |
| Kotlin | [lysine-dev/okhttp](https://github.com/lysine-dev/okhttp) | [1: SocketChannelTest.kt](https://github.com/lysine-dev/okhttp/blob/c3179301fc7d0266c5bf154870c2b0b9da01e7e7/okhttp/src/jvmTest/kotlin/okhttp3/SocketChannelTest.kt), [2: OkHttpFeature.kt](https://github.com/lysine-dev/okhttp/blob/c3179301fc7d0266c5bf154870c2b0b9da01e7e7/okhttp/src/jvmMain/kotlin/okhttp3/internal/graal/OkHttpFeature.kt), [3: TestUtilCommon.kt](https://github.com/lysine-dev/okhttp/blob/c3179301fc7d0266c5bf154870c2b0b9da01e7e7/okhttp-testing-support/src/main/kotlin/okhttp3/TestUtilCommon.kt) | Named objects missing; map/expansion timed out although declaration extraction took 15.5 ms. |
| Ruby | [jekyll/jekyll](https://github.com/jekyll/jekyll) | [1: test_theme_assets_reader.rb](https://github.com/jekyll/jekyll/blob/97c3bdc822df9d5bea4c269b29a28de5a5e29d70/test/test_theme_assets_reader.rb), [2: test_static_file.rb](https://github.com/jekyll/jekyll/blob/97c3bdc822df9d5bea4c269b29a28de5a5e29d70/test/test_static_file.rb), [3: converter.rb](https://github.com/jekyll/jekyll/blob/97c3bdc822df9d5bea4c269b29a28de5a5e29d70/lib/jekyll/converter.rb) | Checked methods, literal require and calls recovered; DSL-generated methods/autoloading remain unverified. |
| PHP | [laravel/framework](https://github.com/laravel/framework) | [1: DatabaseNotificationCollection.php](https://github.com/laravel/framework/blob/5d7797528723107cd4f495326838a7d7d67fbb20/src/Illuminate/Notifications/DatabaseNotificationCollection.php), [2: EntityNotFoundException.php](https://github.com/laravel/framework/blob/5d7797528723107cd4f495326838a7d7d67fbb20/src/Illuminate/Contracts/Queue/EntityNotFoundException.php), [3: DatabaseManager.php](https://github.com/laravel/framework/blob/5d7797528723107cd4f495326838a7d7d67fbb20/src/Illuminate/Database/DatabaseManager.php) | Aliased parent type reference missing; dynamic proxies/macros remain unverified. |
| Zig | [zigtools/zls](https://github.com/zigtools/zls) | [1: tracy.zig](https://github.com/zigtools/zls/blob/432aaf8ed063c49cbb542a9025a5aa87cf876f7d/src/tracy.zig), [2: function.zig](https://github.com/zigtools/zls/blob/432aaf8ed063c49cbb542a9025a5aa87cf876f7d/tests/analysis/function.zig), [3: diff.zig](https://github.com/zigtools/zls/blob/432aaf8ed063c49cbb542a9025a5aa87cf876f7d/tests/utility/diff.zig) | Checked containers, functions, tests and import forms recovered; build-provided modules remain gaps. |
| Bash | [ohmybash/oh-my-bash](https://github.com/ohmybash/oh-my-bash) | [1: robbyrussell.theme.sh](https://github.com/ohmybash/oh-my-bash/blob/4d86087c31b867a30ddaaea270804839b0988e74/themes/robbyrussell/robbyrussell.theme.sh), [2: ansible.plugin.sh](https://github.com/ohmybash/oh-my-bash/blob/4d86087c31b867a30ddaaea270804839b0988e74/plugins/ansible/ansible.plugin.sh), [3: git.plugin.sh](https://github.com/ohmybash/oh-my-bash/blob/4d86087c31b867a30ddaaea270804839b0988e74/plugins/git/git.plugin.sh) | Checked functions/commands recovered; framework loaders and alias execution are outside literal source resolution. |
| Dart | [cfug/dio](https://github.com/cfug/dio) | [1: response_stream_handler.dart](https://github.com/cfug/dio/blob/f1bef462289b40992d625d4eb2a2f42321217573/dio/lib/src/response/response_stream_handler.dart), [2: stacktrace_test.dart](https://github.com/cfug/dio/blob/f1bef462289b40992d625d4eb2a2f42321217573/dio/test/stacktrace_test.dart), [3: io_progress_stream.dart](https://github.com/cfug/dio/blob/f1bef462289b40992d625d4eb2a2f42321217573/dio/lib/src/progress_stream/io_progress_stream.dart) | Nested local function symbols and generic call missing; workspace package imports treated as external. |
| Lua | [neovim/neovim](https://github.com/neovim/neovim) | [1: qf_title_spec.lua](https://github.com/neovim/neovim/blob/187d13aa6047e1914244e8033b2d2f3379a0b97f/test/functional/legacy/qf_title_spec.lua), [2: num_options_spec.lua](https://github.com/neovim/neovim/blob/187d13aa6047e1914244e8033b2d2f3379a0b97f/test/functional/options/num_options_spec.lua), [3: 012_directory_spec.lua](https://github.com/neovim/neovim/blob/187d13aa6047e1914244e8033b2d2f3379a0b97f/test/functional/legacy/012_directory_spec.lua) | Checked local functions and require recovered; runtime paths and aliased receivers remain inferred. |
| Scala | [typelevel/cats](https://github.com/typelevel/cats) | [1: ContTSuite.scala](https://github.com/typelevel/cats/blob/813f1bc8e357991caa587149f1a0ff97cd292d08/tests/shared/src/test/scala/cats/tests/ContTSuite.scala), [2: RenameTupleApplySyntax.scala](https://github.com/typelevel/cats/blob/813f1bc8e357991caa587149f1a0ff97cd292d08/scalafix/v1_0_0/output/src/main/scala/fix/v1_0_0/RenameTupleApplySyntax.scala), [3: RenameCartesian.scala](https://github.com/typelevel/cats/blob/813f1bc8e357991caa587149f1a0ff97cd292d08/scalafix/v1_0_0/input/src/main/scala/fix/v1_0_0/RenameCartesian.scala) | Import paths were truncated to their first segment; local dependency resolution is incomplete. |
| Elixir | [phoenixframework/phoenix](https://github.com/phoenixframework/phoenix) | [1: app_with_mysql_adapter_json_test.exs](https://github.com/phoenixframework/phoenix/blob/e36472d96423be6ae17c5050dfc63c4c6852fcd4/integration_test/test/code_generation/app_with_mysql_adapter_json_test.exs), [2: digester.ex](https://github.com/phoenixframework/phoenix/blob/e36472d96423be6ae17c5050dfc63c4c6852fcd4/lib/phoenix/digester.ex), [3: phx.gen.channel_test.exs](https://github.com/phoenixframework/phoenix/blob/e36472d96423be6ae17c5050dfc63c4c6852fcd4/test/mix/tasks/phx.gen.channel_test.exs) | Checked definitions/calls recovered; private export flags are incorrect and macros remain unexpanded. |
| R | [tidyverse/dplyr](https://github.com/tidyverse/dplyr) | [1: test-when.R](https://github.com/tidyverse/dplyr/blob/d5e94e7fa8fd4a5f79c1a707d1842216bb4c691f/tests/testthat/test-when.R), [2: test-groups-with.R](https://github.com/tidyverse/dplyr/blob/d5e94e7fa8fd4a5f79c1a707d1842216bb4c691f/tests/testthat/test-groups-with.R), [3: select.R](https://github.com/tidyverse/dplyr/blob/d5e94e7fa8fd4a5f79c1a707d1842216bb4c691f/R/select.R) | Checked function assignments and calls recovered; package NAMESPACE and S3 dispatch remain unverified. |
| Julia | [FluxML/Flux.jl](https://github.com/FluxML/Flux.jl) | [1: optstate_reactant.jl](https://github.com/FluxML/Flux.jl/blob/53a969e8132a886468fd030ad19a54e41b30bb92/test/ext_reactant/optstate_reactant.jl), [2: vgg.jl](https://github.com/FluxML/Flux.jl/blob/53a969e8132a886468fd030ad19a54e41b30bb92/perf/layers/vgg.jl), [3: show.jl](https://github.com/FluxML/Flux.jl/blob/53a969e8132a886468fd030ad19a54e41b30bb92/src/layers/show.jl) | Bare using imports omitted; quoted methods are not proof of live definitions. |
| Haskell | [jgm/pandoc](https://github.com/jgm/pandoc) | [1: Lua.hs](https://github.com/jgm/pandoc/blob/b5e35e840ca9fc4e1b11bc9392fea38b749f74c6/pandoc-cli/lua/PandocCLI/Lua.hs), [2: Macros.hs](https://github.com/jgm/pandoc/blob/b5e35e840ca9fc4e1b11bc9392fea38b749f74c6/src/Text/Pandoc/Readers/Mdoc/Macros.hs), [3: Xlsx.hs](https://github.com/jgm/pandoc/blob/b5e35e840ca9fc4e1b11bc9392fea38b749f74c6/test/Tests/Readers/Xlsx.hs) | Calls/types unavailable; point-free function/type-signature omissions and false computed imports observed. |
| Clojure | [ring-clojure/ring](https://github.com/ring-clojure/ring) | [1: project.clj](https://github.com/ring-clojure/ring/blob/6238fe0e2e4506e28a0ce6ade9ae514f7b02cbee/ring-jakarta-servlet/project.clj), [2: cookie.clj](https://github.com/ring-clojure/ring/blob/6238fe0e2e4506e28a0ce6ade9ae514f7b02cbee/ring-core/test/ring/middleware/session/test/cookie.clj), [3: websocket.clj](https://github.com/ring-clojure/ring/blob/6238fe0e2e4506e28a0ce6ade9ae514f7b02cbee/ring-core/test/ring/test/websocket.clj) | Calls/types unavailable and defmethod missing; Java interop imports remain outside this extractor. |
| PowerShell | [pester/Pester](https://github.com/pester/Pester) | [1: Should-NotBeEmptyString.ps1](https://github.com/pester/Pester/blob/98e56d65da1642a7158992b50ded240c42a3ac18/src/functions/assert/String/Should-NotBeEmptyString.ps1), [2: Should-MatchString.ps1](https://github.com/pester/Pester/blob/98e56d65da1642a7158992b50ded240c42a3ac18/src/functions/assert/String/Should-MatchString.ps1), [3: BeNullOrEmpty.Tests.ps1](https://github.com/pester/Pester/blob/98e56d65da1642a7158992b50ded240c42a3ac18/tst/functions/assertions/BeNullOrEmpty.Tests.ps1) | BOM, functions and command calls recovered; static/object method calls are not extracted. |
| Perl | [mojolicious/mojo](https://github.com/mojolicious/mojo) | [1: SyntaxError.pm](https://github.com/mojolicious/mojo/blob/60624d154b1b2081b9229be2c9f3d98af0039e95/t/mojolicious/lib/MojoliciousTest/SyntaxError.pm), [2: Promise.pm](https://github.com/mojolicious/mojo/blob/60624d154b1b2081b9229be2c9f3d98af0039e95/lib/Mojo/Promise.pm), [3: Collection.pm](https://github.com/mojolicious/mojo/blob/60624d154b1b2081b9229be2c9f3d98af0039e95/lib/Mojo/Collection.pm) | Named subroutines/imports recovered; intentional malformed fixture reported a parse gap; POD ignored. |
| SQL | [pgvector/pgvector](https://github.com/pgvector/pgvector) | [1: vector--0.7.3--0.7.4.sql](https://github.com/pgvector/pgvector/blob/5db3087b27a4fd095e7398f4b0f2706bf6d54bd3/sql/vector--0.7.3--0.7.4.sql), [2: sparsevec.sql](https://github.com/pgvector/pgvector/blob/5db3087b27a4fd095e7398f4b0f2706bf6d54bd3/test/sql/sparsevec.sql), [3: vector--0.1.5--0.1.6.sql](https://github.com/pgvector/pgvector/blob/5db3087b27a4fd095e7398f4b0f2706bf6d54bd3/sql/vector--0.1.5--0.1.6.sql) | psql guards and PostgreSQL expressions produced parse gaps; no function-call extraction. |
| HCL | [terraform-aws-modules/terraform-aws-vpc](https://github.com/terraform-aws-modules/terraform-aws-vpc) | [1: outputs.tf](https://github.com/terraform-aws-modules/terraform-aws-vpc/blob/b3abd6df2ecf052451a361ed55b8f06f8742a795/examples/separate-route-tables/outputs.tf), [2: variables.tf](https://github.com/terraform-aws-modules/terraform-aws-vpc/blob/b3abd6df2ecf052451a361ed55b8f06f8742a795/wrappers/vpc-endpoints/variables.tf), [3: versions.tf](https://github.com/terraform-aws-modules/terraform-aws-vpc/blob/b3abd6df2ecf052451a361ed55b8f06f8742a795/examples/ipv6-dualstack/versions.tf) | Output/variable/nested blocks recovered; sibling module context available, provider execution unverified. |
| HTML | [mdn/dom-examples](https://github.com/mdn/dom-examples) | [1: index.html](https://github.com/mdn/dom-examples/blob/03ae1ffb325a9202d3a64f26ab5b8361392a5368/history-api/index.html), [2: index.html](https://github.com/mdn/dom-examples/blob/03ae1ffb325a9202d3a64f26ab5b8361392a5368/scroll-promises/window-methods/index.html), [3: index3.html](https://github.com/mdn/dom-examples/blob/03ae1ffb325a9202d3a64f26ab5b8361392a5368/view-transitions/mpa-chapter-nav-transition-types/index3.html) | Script/style dependencies recovered; self-closing image ID missing from map. |
| CSS | [animate-css/animate.css](https://github.com/animate-css/animate.css) | [1: rubberBand.css](https://github.com/animate-css/animate.css/blob/3f8ab233dbbd9d2fe577528d2296382954be3d1a/source/attention_seekers/rubberBand.css), [2: backOutRight.css](https://github.com/animate-css/animate.css/blob/3f8ab233dbbd9d2fe577528d2296382954be3d1a/source/back_exits/backOutRight.css), [3: slideOutDown.css](https://github.com/animate-css/animate.css/blob/3f8ab233dbbd9d2fe577528d2296382954be3d1a/source/sliding_exits/slideOutDown.css) | Selectors recovered and keyframes preserved in source; keyframes are not separately indexed. |

## Reproduce

Requires authenticated `gh`, Python 3, and the repository's Rust toolchain.
Use an external cache/output directory; downloaded source is not checked in.

```bash
cargo build --release --locked --bin context-creator --example audit_language_support
python3 benchmarks/oss_audit.py prepare --cache /tmp/context-creator-oss-cache
python3 benchmarks/oss_audit.py run --cache /tmp/context-creator-oss-cache
python3 benchmarks/oss_audit.py probe --cache /tmp/context-creator-oss-cache
python3 benchmarks/oss_audit.py probe --mode sessions --cache /tmp/context-creator-oss-cache
python3 benchmarks/oss_audit.py probe --mode symbols --language Java --cache /tmp/context-creator-oss-cache
python3 benchmarks/oss_audit.py probe --mode symbols --language Kotlin --cache /tmp/context-creator-oss-cache
python3 benchmarks/oss_report.py --raw /tmp/context-creator-oss-results \
  --output /tmp/context-creator-oss-summary.json
```

`prepare` reuses the checked-in commits, seed, and selected files; it verifies
candidate-pool fingerprints. `run`/`probe` verify selected content hashes before
work. Repeated probes remove prior results for that mode before running, so a
timeout cannot reuse a stale success. Failures remain recorded in raw JSON, so a successful harness exit
means the measurement finished, not that every product feature passed.

The current harness uses a child-only neutral Git configuration by default.
`--user-git-config` reproduces machine-specific ignore effects from the initial
run. The committed receipt distinguishes that initial environment from the
supplemental neutral SQL/source-page run. Session edits affect only the external
cache and are restored, including when a probe times out.

## Boundaries and next work

This audit covers source-reading semantics and the shared MCP read helper on
Linux. Actual MCP transport behavior is covered by the repository's separate
integration suite. It does not execute compiler/build resolution, evaluate
runtime dispatch, exercise Git-history change-impact recipes on the archives,
or measure live model answer quality. Larger files, other dialects and unselected
syntax forms need more coverage.

The immediate priorities are valid confined relative imports; complete import
extraction; one reusable project index and bounded semantic work; missing
function/type/call forms; feature-level capability reporting; and consistent
ignore policy across entry points. The pinned failures provide concrete inputs
for that work.
