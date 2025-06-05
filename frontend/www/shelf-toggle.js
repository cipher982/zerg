// Simple client-side script to toggle the mobile agent shelf.

document.addEventListener('DOMContentLoaded', () => {
  const btn = document.getElementById('shelf-toggle-btn');
  const shelf = () => document.getElementById('agent-shelf');

  if (btn) {
    btn.addEventListener('click', () => {
      const el = shelf();
      if (el) {
        el.classList.toggle('open');
      }
    });
  }
});
