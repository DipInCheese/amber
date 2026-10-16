/* Timeline scrollbar for Amber's HTML export. Self-contained: no libraries,
 * no network, and it never reads message text - it only looks at the
 * <section class="day" data-ts="..."> wrappers the exporter writes. */
(function () {
  "use strict";

  var sections = document.querySelectorAll("section.day");
  var timeline = document.getElementById("timeline");
  var track = document.getElementById("tl-track");
  var thumb = document.getElementById("tl-thumb");
  var label = document.getElementById("tl-label");
  var header = document.querySelector("header");
  if (!sections.length || !timeline || !track || !thumb || !label) return;

  var days = [];
  for (var i = 0; i < sections.length; i++) {
    days.push({ el: sections[i], ts: Number(sections[i].getAttribute("data-ts")) });
  }

  var DAY_MS = 86400000;
  var minTs = days[0].ts;
  var maxTs = days[days.length - 1].ts + DAY_MS; // non-zero span even for a single day
  var span = maxTs - minTs;

  var monthFormat = new Intl.DateTimeFormat(undefined, { month: "long", year: "numeric" });
  var reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  document.documentElement.classList.add("has-timeline");

  // --- Math ---------------------------------------------------------------

  function clamp(value, low, high) {
    return Math.min(high, Math.max(low, value));
  }

  function fractionOf(ts) {
    return span > 0 ? clamp((ts - minTs) / span, 0, 1) : 0;
  }

  function tsAt(fraction) {
    return minTs + clamp(fraction, 0, 1) * span;
  }

  // First day with messages at or after `ts` (the last day when `ts` is past the end).
  function dayAtOrAfter(ts) {
    var low = 0;
    var high = days.length - 1;
    while (low < high) {
      var mid = (low + high) >> 1;
      if (days[mid].ts < ts) low = mid + 1;
      else high = mid;
    }
    return low;
  }

  function headerHeight() {
    return header ? header.offsetHeight : 0;
  }

  // The day whose section is at (or just above) the top of the visible page.
  function currentDay() {
    var limit = headerHeight() + 2;
    var low = 0;
    var high = days.length - 1;
    while (low < high) {
      var mid = (low + high + 1) >> 1;
      if (days[mid].el.getBoundingClientRect().top <= limit) low = mid;
      else high = mid - 1;
    }
    return low;
  }

  function scrollTargetFor(index) {
    var top = days[index].el.getBoundingClientRect().top + window.pageYOffset - headerHeight();
    var max = document.documentElement.scrollHeight - window.innerHeight;
    return clamp(top, 0, Math.max(0, max));
  }

  // --- Ticks ----------------------------------------------------------------

  function buildTicks() {
    var start = new Date(minTs);
    var year = start.getFullYear();
    var month = start.getMonth() + 1; // first month boundary after the first message
    var fragment = document.createDocumentFragment();
    for (;;) {
      var boundary = new Date(year, month, 1).getTime();
      if (boundary >= maxTs) break;
      var tick = document.createElement("div");
      tick.style.top = fractionOf(boundary) * 100 + "%";
      var isYear = new Date(boundary).getMonth() === 0;
      tick.className = isYear ? "tl-tick tl-tick-year" : "tl-tick";
      if (isYear) {
        var text = document.createElement("span");
        text.textContent = String(new Date(boundary).getFullYear());
        tick.appendChild(text);
      }
      fragment.appendChild(tick);
      month += 1;
    }
    track.insertBefore(fragment, thumb);
  }
  buildTicks();

  // --- Thumb --------------------------------------------------------------

  var trackHeight = 0;
  var dragging = false;
  var shownLabel = "";

  function measure() {
    // Start the strip just below the sticky header, wherever it wraps to.
    timeline.style.setProperty("--tl-top", headerHeight() + 8 + "px");
    trackHeight = track.clientHeight;
  }

  function placeThumb(fraction) {
    thumb.style.transform = "translate3d(0," + fraction * trackHeight + "px,0)";
  }

  function setLabel(ts) {
    var text = monthFormat.format(new Date(ts));
    if (text === shownLabel) return;
    shownLabel = text;
    label.textContent = text;
    track.setAttribute("aria-valuetext", text);
  }

  var syncQueued = false;
  function syncToScroll() {
    syncQueued = false;
    if (dragging) return;
    var ts = days[currentDay()].ts;
    placeThumb(fractionOf(ts));
    setLabel(ts);
  }

  function queueSync() {
    if (syncQueued) return;
    syncQueued = true;
    requestAnimationFrame(syncToScroll);
  }

  // --- Gliding to a day ---------------------------------------------------

  var TAU_MS = 95;
  var SNAP_PX = 0.75;
  var chaseIndex = -1;
  var chaseFrame = 0;
  var chaseLast = 0;
  var chaseSettled = 0;

  // Same exponential ease as the app: framerate independent, never overshoots.
  function ease(current, target, dtMs) {
    var remaining = target - current;
    if (Math.abs(remaining) <= SNAP_PX) return target;
    var next = current + remaining * (1 - Math.exp(-Math.max(0, dtMs) / TAU_MS));
    return Math.abs(target - next) <= SNAP_PX ? target : next;
  }

  function stopChase() {
    if (chaseFrame) cancelAnimationFrame(chaseFrame);
    chaseFrame = 0;
    chaseIndex = -1;
  }

  function chaseStep(now) {
    if (chaseIndex < 0) return stopChase();
    var dt = Math.min(50, Math.max(1, now - chaseLast));
    chaseLast = now;
    var target = scrollTargetFor(chaseIndex);
    var next = ease(window.pageYOffset, target, dt);
    window.scrollTo(0, next);
    // Layout shifts as nearby sections render, so only finish after two
    // consecutive frames sitting on the target.
    chaseSettled = next === target ? chaseSettled + 1 : 0;
    if (chaseSettled >= 2) return stopChase();
    chaseFrame = requestAnimationFrame(chaseStep);
  }

  function glideToDay(index) {
    if (reducedMotion) {
      window.scrollTo(0, scrollTargetFor(index));
      return;
    }
    var wasIdle = !chaseFrame;
    chaseIndex = index;
    chaseSettled = 0;
    if (wasIdle) {
      chaseLast = performance.now();
      chaseFrame = requestAnimationFrame(chaseStep);
    }
  }

  // The reader taking over (wheel, touch, keys in the page) beats a glide.
  function userTookOver(event) {
    if (track.contains(event.target)) return;
    stopChase();
  }
  window.addEventListener("wheel", userTookOver, { passive: true });
  window.addEventListener("touchstart", userTookOver, { passive: true });
  window.addEventListener("keydown", userTookOver, { passive: true });

  // --- Dragging -------------------------------------------------------------

  var pendingY = null;
  var dragFrame = 0;

  function applyPointer() {
    dragFrame = 0;
    if (pendingY === null) return;
    var rect = track.getBoundingClientRect();
    var fraction = rect.height > 0 ? clamp((pendingY - rect.top) / rect.height, 0, 1) : 0;
    pendingY = null;
    var ts = tsAt(fraction);
    placeThumb(fraction);
    setLabel(ts);
    glideToDay(dayAtOrAfter(ts));
  }

  function queuePointer(clientY) {
    pendingY = clientY;
    if (!dragFrame) dragFrame = requestAnimationFrame(applyPointer);
  }

  function endDrag() {
    if (!dragging) return;
    if (pendingY !== null) applyPointer(); // a quick tap can end before the next frame
    if (dragFrame) cancelAnimationFrame(dragFrame);
    dragFrame = 0;
    pendingY = null;
    dragging = false;
    timeline.classList.remove("is-dragging");
    queueSync();
  }

  track.addEventListener("pointerdown", function (event) {
    if (event.button !== 0) return;
    try {
      track.setPointerCapture(event.pointerId);
    } catch (error) {
      /* pointer already gone; the move events still arrive */
    }
    dragging = true;
    timeline.classList.add("is-dragging");
    queuePointer(event.clientY);
  });
  track.addEventListener("pointermove", function (event) {
    if (dragging) queuePointer(event.clientY);
  });
  track.addEventListener("pointerup", endDrag);
  track.addEventListener("pointercancel", endDrag);
  track.addEventListener("lostpointercapture", endDrag);

  // Keyboard: arrows move a month, Page keys a year.
  track.addEventListener("keydown", function (event) {
    var step =
      event.key === "ArrowUp" ? -1 : event.key === "ArrowDown" ? 1 : event.key === "PageUp" ? -12 : event.key === "PageDown" ? 12 : 0;
    if (!step) return;
    event.preventDefault();
    var here = new Date(days[currentDay()].ts);
    var moved = new Date(here.getFullYear(), here.getMonth() + step, 1).getTime();
    glideToDay(dayAtOrAfter(moved));
  });

  window.addEventListener("scroll", queueSync, { passive: true });
  window.addEventListener("resize", function () {
    measure();
    queueSync();
  });

  measure();
  syncToScroll();
})();
