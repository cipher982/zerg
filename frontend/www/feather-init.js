/* ------------------------------------------------------------------
   Initialise Feather icons once the DOM and external script are loaded.
   The library automatically replaces any element with [data-feather]
   attribute with an inline SVG that inherits `currentColor`.
------------------------------------------------------------------ */

document.addEventListener('DOMContentLoaded', () => {
  if (window.feather && typeof window.feather.replace === 'function') {
    window.feather.replace({
      'stroke-width': 1.5,
    });
  }
});
