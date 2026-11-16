// HLS playback in the browser.
//
// Tracks are streamed as HLS playlists (.m3u8), relayed by the backend. The
// audio plugin used by the app (just_audio) only ever does
// `audioElement.src = url`, which plays HLS in Safari and nowhere else
// reliably. This file teaches media elements that an assignment of an HLS URL
// means "play it through hls.js" (Media Source Extensions).
//
// Apple browsers keep their native player. Nothing changes for other URLs.
// Requires vendor/hls.light.min.js to be loaded first.
(function () {
  'use strict';

  var Hls = window.Hls;
  var proto = window.HTMLMediaElement && window.HTMLMediaElement.prototype;
  var srcProperty = proto && Object.getOwnPropertyDescriptor(proto, 'src');
  if (!Hls || !Hls.isSupported() || !srcProperty || !srcProperty.set) {
    return;
  }

  var nativeLoad = proto.load;
  var nativeRemoveAttribute = proto.removeAttribute;

  // Media element -> the hls.js instance currently feeding it.
  var players = new WeakMap();
  // Elements hls.js itself is attaching to or detaching from right now. Its
  // own calls to `src`, `load()` and `removeAttribute()` must go straight to
  // the browser.
  var internal = new WeakSet();

  function isHlsUrl(url) {
    return typeof url === 'string' && /\.m3u8($|[?#])/i.test(url);
  }

  // Safari's HLS player is the reference implementation, and on iPhone it is
  // the only option. Other browsers either cannot play HLS at all (Firefox)
  // or have a young native player (recent Chromium): hls.js behaves the same
  // everywhere, and the native player remains the fallback if it fails.
  function prefersNativeHls(element) {
    return (
      /Apple/.test(navigator.vendor || '') &&
      element.canPlayType('application/vnd.apple.mpegurl') !== ''
    );
  }

  function asInternal(element, action) {
    internal.add(element);
    try {
      action();
    } finally {
      internal.delete(element);
    }
  }

  function teardown(element) {
    var hls = players.get(element);
    if (!hls) return;
    players.delete(element);
    asInternal(element, function () {
      hls.destroy();
    });
  }

  function startHls(element, url) {
    var hls = new Hls();
    players.set(element, hls);

    hls.on(Hls.Events.ERROR, function (event, data) {
      if (!data.fatal || players.get(element) !== hls) return;

      if (data.type === Hls.ErrorTypes.MEDIA_ERROR && !hls.triedMediaRecovery) {
        hls.triedMediaRecovery = true;
        asInternal(element, function () {
          hls.recoverMediaError();
        });
        return;
      }

      // Unrecoverable: hand the URL to the browser so the element reports a
      // regular media error, which the app already knows how to surface.
      console.error('HLS playback failed', data.type, data.details);
      teardown(element);
      srcProperty.set.call(element, url);
      nativeLoad.call(element);
    });

    hls.loadSource(url);
    asInternal(element, function () {
      hls.attachMedia(element);
    });
  }

  Object.defineProperty(proto, 'src', {
    configurable: true,
    enumerable: srcProperty.enumerable,
    get: function () {
      return srcProperty.get.call(this);
    },
    set: function (url) {
      if (internal.has(this)) {
        srcProperty.set.call(this, url);
        return;
      }

      teardown(this);

      if (isHlsUrl(url) && !prefersNativeHls(this)) {
        startHls(this, url);
      } else {
        srcProperty.set.call(this, url);
      }
    },
  });

  proto.load = function () {
    // While hls.js feeds the element, reloading it would detach the media
    // source it is filling. The stream is already loading.
    if (players.has(this) && !internal.has(this)) return;
    return nativeLoad.call(this);
  };

  proto.removeAttribute = function (name) {
    if (
      !internal.has(this) &&
      typeof name === 'string' &&
      name.toLowerCase() === 'src'
    ) {
      teardown(this);
    }
    return nativeRemoveAttribute.call(this, name);
  };
})();
