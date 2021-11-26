import '../lib/bindings.dart';
import 'dart:ffi';
import 'dart:isolate';

void main() {
  final api = Api.load();
  api.hello_world();

  final rx = ReceivePort()..listen(onMessage);

  final registerCallbackPtr = api.lookup<
    NativeFunction<
    Void Function(
      Int64,
      Pointer<Void>,
      //Pointer<NativeFunction<Int8 Function(Int64, Pointer<Dart_CObject>)>>,
    )>>('register_callback');
  final registerCallback = registerCallbackPtr.asFunction<
    void Function(
      int,
      Pointer<Void>,
      //Pointer<NativeFunction<Int8 Function(Int64, Pointer<Dart_CObject>)>>,
    )>();
  registerCallback(rx.sendPort.nativePort, NativeApi.postCObject.cast());
}

void onMessage(dynamic message) {
  print('onMessage');
}
