import '../lib/bindings.dart';
import 'dart:ffi';
import 'dart:isolate';

Future<T> nativeFuture(
  Api api,
  Box box,
  Function<T? Function(Box)> poll,
) {
  final completer = Completer();
  final rx = ReceivePort();
  rx.listen((dynamic _message) {
    final ret = poll(box, rx.sendPort.nativePort, NativeApi.postCObject.cast());
    if (ret != null) {
      rx.close();
      completer.complete(ret);
    }
  });
  return completer.future;
}

void main() async {
  final api = Api.load();
  api.hello_world();

  final res = await nativeFuture(api, api.create_future(), api.poll_future);
  print(res);

  /*final registerCallbackPtr =
      api.lookup<NativeFunction<Void Function(Int64, Pointer<Void>)>>(
          'register_callback');
  final registerCallback =
      registerCallbackPtr.asFunction<void Function(int, Pointer<Void>)>();
  registerCallback(closure.port(), NativeApi.postCObject.cast());*/
}
