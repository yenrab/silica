/* Silica documentation site — progressive enhancement only.
   With JavaScript off the page is still complete and readable; this
   file adds the contents rail, heading anchors, code labels, and the
   light/dark toggle. */
(function () {
  "use strict";

  var content = document.getElementById("content");

  /* ---------------------------------------------------------
     Theme toggle. The pre-paint script in the layout has
     already applied any stored choice; this only switches it.
     --------------------------------------------------------- */
  var toggle = document.getElementById("theme-toggle");
  if (toggle) {
    toggle.addEventListener("click", function () {
      var root = document.documentElement;
      var explicit = root.getAttribute("data-theme");
      var isDark = explicit
        ? explicit === "dark"
        : window.matchMedia("(prefers-color-scheme: dark)").matches;
      var next = isDark ? "light" : "dark";

      root.setAttribute("data-theme", next);
      try {
        localStorage.setItem("silica-theme", next);
      } catch (e) { /* private mode: the choice just won't persist */ }
    });
  }

  if (!content) return;

  /* ---------------------------------------------------------
     Contents rail, built from the headings actually on the page.
     Kramdown gives every heading an id, so these are stable
     links that match the anchors used elsewhere in the docs.
     --------------------------------------------------------- */
  var toc = document.getElementById("toc");
  var tocNav = document.getElementById("toc-nav");
  var headings = [];

  if (toc && tocNav) {
    var found = content.querySelectorAll("h2[id], h3[id]");
    var h2Count = content.querySelectorAll("h2[id]").length;

    // With only one or two sections a rail is noise, not navigation.
    if (h2Count >= 3) {
      // Long pages list H2 only; short ones can afford H3 detail too.
      var includeH3 = h2Count <= 8;

      Array.prototype.forEach.call(found, function (h) {
        if (h.tagName === "H3" && !includeH3) return;

        var a = document.createElement("a");
        a.href = "#" + h.id;
        a.textContent = h.textContent.replace(/\s*¶\s*$/, "").trim();
        if (h.tagName === "H3") a.className = "lvl-3";
        tocNav.appendChild(a);
        headings.push({ el: h, link: a });
      });

      if (headings.length) toc.hidden = false;
    }
  }

  /* ---- scrollspy: mark the section the reader is in ---- */
  if (headings.length && "IntersectionObserver" in window) {
    var visible = [];

    var observer = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        var id = entry.target.id;
        var at = visible.indexOf(id);
        if (entry.isIntersecting) {
          if (at === -1) visible.push(id);
        } else if (at !== -1) {
          visible.splice(at, 1);
        }
      });

      var currentId = null;
      if (visible.length) {
        // Topmost heading currently on screen.
        currentId = headings
          .filter(function (h) { return visible.indexOf(h.el.id) !== -1; })
          .map(function (h) { return h.el.id; })[0];
      } else {
        // Between headings: the last one scrolled past.
        var above = headings.filter(function (h) {
          return h.el.getBoundingClientRect().top < 120;
        });
        if (above.length) currentId = above[above.length - 1].el.id;
      }

      headings.forEach(function (h) {
        h.link.classList.toggle("is-current", h.el.id === currentId);
      });
    }, { rootMargin: "-70px 0px -70% 0px", threshold: 0 });

    headings.forEach(function (h) { observer.observe(h.el); });
  }

  /* ---------------------------------------------------------
     Heading anchors — this site links to its own sections a lot.
     --------------------------------------------------------- */
  Array.prototype.forEach.call(content.querySelectorAll("h2[id], h3[id]"), function (h) {
    var a = document.createElement("a");
    a.className = "heading-anchor";
    a.href = "#" + h.id;
    a.textContent = "#";
    a.setAttribute("aria-label", "Link to this section");
    h.appendChild(a);
  });

  /* ---------------------------------------------------------
     Label each code block with its language.

     Only direct children of .content are considered: kramdown gives
     INLINE code spans the same `highlighter-rouge` class, and matching
     those would drop a label inside every `word` in the prose.
     --------------------------------------------------------- */
  Array.prototype.slice.call(content.children).forEach(function (node) {
    var lang = null;

    if (node.tagName === "PRE") {
      // No Rouge lexer for Silica: a bare <pre><code class="language-x">.
      var inner = node.querySelector("code[class*='language-']");
      if (inner) lang = /language-([\w+-]+)/.exec(inner.className);
    } else if (node.tagName === "DIV" && node.classList.contains("highlighter-rouge")) {
      lang = /language-([\w+-]+)/.exec(node.className);
    }

    if (!lang || lang[1] === "plaintext") return;

    var label = document.createElement("div");
    label.className = "code-label";
    label.textContent = lang[1];

    if (node.tagName === "PRE") {
      // Wrap so the label sits above the scroll area, not inside it.
      var wrap = document.createElement("div");
      wrap.className = "code-block";
      node.parentNode.insertBefore(wrap, node);
      wrap.appendChild(label);
      wrap.appendChild(node);
    } else {
      node.insertBefore(label, node.firstChild);
    }
  });
})();
