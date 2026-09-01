/**
 * The single place path parameters are URI-encoded.
 *
 * Every dynamic segment of a request path (`market_id`, `tx_hash`, `user`,
 * `template_name`, `job_id`, ...) must go through here. A value containing `/`, `?`, or
 * `#` that skips encoding corrupts the URL or routes the request to the wrong endpoint
 * (regression trap, commit `dd027a9`). Keeping encoding in one helper also means it is
 * never applied twice in a call chain.
 *
 * No other module under `src/lib/api/` should call `encodeURIComponent` directly - a
 * test enforces this.
 */

/** Fill a single `{placeholder}` segment of a path template with a URI-encoded value. */
export function fillPath(
  template: string,
  placeholder: string,
  value: string | number,
): string {
  return template.replace(`{${placeholder}}`, encodeURIComponent(value));
}

/** Fill every `{placeholder}` in a template from a map of raw (unencoded) values. */
export function fillPathParams(
  template: string,
  params: Record<string, string | number>,
): string {
  return Object.entries(params).reduce(
    (path, [placeholder, value]) => fillPath(path, placeholder, value),
    template,
  );
}
