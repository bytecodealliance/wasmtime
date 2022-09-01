---
name: Fuzz Bug Report
about: Report a fuzz bug in Wasmtime or Cranelift
title: '<target> fuzzbug: '
labels: bug, fuzz-bug
assignees: ''
---

Thanks for filing an issue! Please fill out the TODOs below, and change `<target>` in the title to the corresponding fuzzing target.

<!-- TODO: add link to an external bug report, if there is one, such as from OSS-Fuzz -->

<details>
<summary>Test case input</summary>

<!-- Please base64-encode the input that libFuzzer generated, and paste it in the code-block below. This is required for us to reproduce the issue. -->

```
TODO_paste_the_base64_encoded_input_here
```

</details>

<details>
<summary>`cargo +nightly fuzz fmt` output</summary>

<!-- If you can, please paste the output of `cargo +nightly fuzz fmt <target> <input>` in the code-block below. This will help reviewers more quickly triage this report. -->

```
TODO_paste_cargo_fuzz_fmt_output_here
```

</details>

<details>
<summary>Stack trace or other relevant details</summary>

<!-- If you can, please paste anything that looks relevant from the failure message in the code-block below. This will help reviewers more quickly triage this report. -->

```
TODO_paste_the_report_here
```

</details>
