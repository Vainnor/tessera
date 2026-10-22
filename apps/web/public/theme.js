// The site's only script. It runs in <head>, before first paint, so a saved theme applies
// without a flash; then it reveals the footer theme picker, which needs JavaScript.
// The valid ids come from <meta name="tessera-themes">, rendered from the theme package.
(function () {
  "use strict";
  var KEY = "tessera.theme";
  var root = document.documentElement;
  var meta = document.querySelector('meta[name="tessera-themes"]');
  var ids = meta ? meta.getAttribute("content").split(" ") : [];

  function read() {
    try {
      var saved = window.localStorage.getItem(KEY);
      return ids.indexOf(saved) >= 0 ? saved : "auto";
    } catch (e) {
      return "auto";
    }
  }

  function apply(choice) {
    if (choice === "auto") delete root.dataset.theme;
    else root.dataset.theme = choice;
  }

  apply(read());

  document.addEventListener("DOMContentLoaded", function () {
    var picker = document.querySelector("[data-theme-picker]");
    var wrap = document.querySelector("[data-theme-picker-wrap]");
    if (!picker || !wrap) return;
    picker.value = read();
    wrap.hidden = false;
    picker.addEventListener("change", function () {
      var choice = ids.indexOf(picker.value) >= 0 ? picker.value : "auto";
      apply(choice);
      try {
        if (choice === "auto") window.localStorage.removeItem(KEY);
        else window.localStorage.setItem(KEY, choice);
      } catch (e) {
        // Storage blocked: the theme still applies for this page.
      }
    });
  });
})();
