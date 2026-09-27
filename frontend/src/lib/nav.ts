/**
 * Shared helpers for navigation link state.
 *
 * Kept dependency-free so both the admin and non-admin layouts can import it
 * without creating a coupling between those modules.
 */

/**
 * Returns true when `href` matches the current `pathname`.
 *
 * A nav item is considered active when the pathname is exactly the href or is
 * nested beneath it (e.g. `/admin/users/42` matches `/admin/users`).
 */
export function isNavItemActive(
  pathname: string | null | undefined,
  href: string,
): boolean {
  if (!pathname || !href) {
    return false;
  }

  return pathname === href || pathname.startsWith(href + '/');
}
