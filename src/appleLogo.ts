export const BUILT_IN_APPLE_LOGO_DATA_URL =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAB4AAAAeCAMAAAAM7l6QAAAAAXNSR0IB2cksfwAAAAlwSFlzAAALEwAACxMBAJqcGAAAAF1QTFRFAAAAVLg1O4YkGkMNpMtzRERE1v6kkvx6M3Uf985V//////9U8Z5Lkmg8v2sn///R7XAt986gqyMX8Z7KyysdfBYNXQ4HvECW6jOXjTqUXzZjAACjQGXFAACCEjKTdwg/iQAAAB90Uk5TAP///////////////////////////////////////0dJnRUAAACkSURBVHicndLBCsIwDAbgP50KXjx4URDf/9VEvXkRi6tZ2nRVGPvnRtcsX0hHV8HvJXr348siFgQriH9xMI5TzSdYbPRfEcsBK41eNbPBOy9A8LY0hLXsLH7qM9KcF8lckeJd5VyQlJtNneVOU6gFQ/yged80Hz/vSjJwaDjZJgtuvi0Ee+SYcKcZOJXf4kfiAixg4KxjSB7L3JwWgoF1Sfo8zx/YV04fORY93QAAAABJRU5ErkJggg==";

const LEGACY_APPLE_LOGO_SUFFIX = "/piforma-panel/apple-color.png";

export function shouldUseBuiltInAppleLogo(configuredPath: string): boolean {
  const normalizedPath = configuredPath.trim().replace(/\\/g, "/");
  return normalizedPath.length === 0 || normalizedPath.endsWith(LEGACY_APPLE_LOGO_SUFFIX);
}
