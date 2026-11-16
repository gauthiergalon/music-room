// Google's sign-in button only exists on the web; the import of its plugin
// must not reach the other platforms.
export 'gsi_button_stub.dart'
    if (dart.library.js_interop) 'gsi_button_web.dart';
