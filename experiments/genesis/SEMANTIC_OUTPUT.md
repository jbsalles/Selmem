# GENESIS semantic output correction 007

Semantic interpretation requests now include a strict JSON Schema with all 15 required annotation fields, numeric types for numeric fields, and no extra properties. Field names, prompts, model settings, salience thresholds, and organ validation rules are unchanged. This constrains transport formatting; it does not certify semantic accuracy.

The decoder also accepts a numeric value serialized as a string, such as `"0.25"`, preserving its value. Nulls, booleans, arrays, objects, empty or nonnumeric strings, non-finite values, and out-of-range numbers remain invalid. No default values or clamping are introduced. Errors report the field and received value.

An invalid returned annotation is requested again with the identical request, up to three annotation attempts in total. Every response and validation error is audited. These calls and transport-level retries share the cumulative HTTP budget. Transport errors use the existing retry policy; they do not trigger another annotation-level retry loop. Invalid annotations are never cached. Invalid or corrupt cache entries stop the operation without silently fetching a replacement. A terminal failure prevents saving the candidate memory state.

Structured response formats are documented at https://docs.x.ai/developers/model-capabilities/text/structured-outputs . Endpoints used for semantic interpretation must support `response_format: json_schema`; unsupported endpoints fail explicitly rather than silently weakening the request.

## Restart after the event-3 failure

The earlier failure log establishes that the returned `arousal` field was not a JSON number, but it does not show its raw value. The raw response is retained in the failed run's `http.jsonl`; a quoted number is one possible cause, not an established diagnosis without that response.

This correction changes both code and the annotation request. Existing resume checks intentionally reject mixing versions. Do not override their checksums or edit the old run design. Keep the failed directory intact and start a fresh collection:

```bash
bash experiments/genesis/run-creative-preview.sh \
  experiments/genesis/creative-preview.json \
  experiments/genesis/results/creative-preview-002
```

The two previously completed events will be re-encoded. The former output directory is not modified. Subsequent pauses within the corrected version can use the same command with `--resume`.

The preview, economy, and full fixtures and creative generation tasks are unchanged. Do not pool partial annotation collections across this request revision.
