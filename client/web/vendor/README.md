Third-party scripts shipped with the web build.

- `hls.light.min.js`: [hls.js](https://github.com/video-dev/hls.js) 1.7.4,
  light build, unmodified (`dist/hls.light.min.js` of the npm package).
  Apache-2.0, see `hls.js.LICENSE`. Used by `../hls_audio.js` to play HLS
  streams in browsers without native support (Chrome, Firefox).

To update: `npm pack hls.js@<version>` and copy `dist/hls.light.min.js` here.
