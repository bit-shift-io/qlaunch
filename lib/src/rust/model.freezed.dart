// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'model.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$ShortcutPayload {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is ShortcutPayload);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'ShortcutPayload()';
}


}

/// @nodoc
class $ShortcutPayloadCopyWith<$Res>  {
$ShortcutPayloadCopyWith(ShortcutPayload _, $Res Function(ShortcutPayload) __);
}


/// Adds pattern-matching-related methods to [ShortcutPayload].
extension ShortcutPayloadPatterns on ShortcutPayload {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ShortcutPayload_LockScreen value)?  lockScreen,TResult Function( ShortcutPayload_Notifications value)?  notifications,TResult Function( ShortcutPayload_Android value)?  android,TResult Function( ShortcutPayload_Command value)?  command,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ShortcutPayload_LockScreen() when lockScreen != null:
return lockScreen(_that);case ShortcutPayload_Notifications() when notifications != null:
return notifications(_that);case ShortcutPayload_Android() when android != null:
return android(_that);case ShortcutPayload_Command() when command != null:
return command(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ShortcutPayload_LockScreen value)  lockScreen,required TResult Function( ShortcutPayload_Notifications value)  notifications,required TResult Function( ShortcutPayload_Android value)  android,required TResult Function( ShortcutPayload_Command value)  command,}){
final _that = this;
switch (_that) {
case ShortcutPayload_LockScreen():
return lockScreen(_that);case ShortcutPayload_Notifications():
return notifications(_that);case ShortcutPayload_Android():
return android(_that);case ShortcutPayload_Command():
return command(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ShortcutPayload_LockScreen value)?  lockScreen,TResult? Function( ShortcutPayload_Notifications value)?  notifications,TResult? Function( ShortcutPayload_Android value)?  android,TResult? Function( ShortcutPayload_Command value)?  command,}){
final _that = this;
switch (_that) {
case ShortcutPayload_LockScreen() when lockScreen != null:
return lockScreen(_that);case ShortcutPayload_Notifications() when notifications != null:
return notifications(_that);case ShortcutPayload_Android() when android != null:
return android(_that);case ShortcutPayload_Command() when command != null:
return command(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  lockScreen,TResult Function()?  notifications,TResult Function( String? action,  String? data,  String? mimeType,  List<String> categories,  String? package,  String? component,  int flags,  List<Extra> extras)?  android,TResult Function( String command)?  command,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ShortcutPayload_LockScreen() when lockScreen != null:
return lockScreen();case ShortcutPayload_Notifications() when notifications != null:
return notifications();case ShortcutPayload_Android() when android != null:
return android(_that.action,_that.data,_that.mimeType,_that.categories,_that.package,_that.component,_that.flags,_that.extras);case ShortcutPayload_Command() when command != null:
return command(_that.command);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  lockScreen,required TResult Function()  notifications,required TResult Function( String? action,  String? data,  String? mimeType,  List<String> categories,  String? package,  String? component,  int flags,  List<Extra> extras)  android,required TResult Function( String command)  command,}) {final _that = this;
switch (_that) {
case ShortcutPayload_LockScreen():
return lockScreen();case ShortcutPayload_Notifications():
return notifications();case ShortcutPayload_Android():
return android(_that.action,_that.data,_that.mimeType,_that.categories,_that.package,_that.component,_that.flags,_that.extras);case ShortcutPayload_Command():
return command(_that.command);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  lockScreen,TResult? Function()?  notifications,TResult? Function( String? action,  String? data,  String? mimeType,  List<String> categories,  String? package,  String? component,  int flags,  List<Extra> extras)?  android,TResult? Function( String command)?  command,}) {final _that = this;
switch (_that) {
case ShortcutPayload_LockScreen() when lockScreen != null:
return lockScreen();case ShortcutPayload_Notifications() when notifications != null:
return notifications();case ShortcutPayload_Android() when android != null:
return android(_that.action,_that.data,_that.mimeType,_that.categories,_that.package,_that.component,_that.flags,_that.extras);case ShortcutPayload_Command() when command != null:
return command(_that.command);case _:
  return null;

}
}

}

/// @nodoc


class ShortcutPayload_LockScreen extends ShortcutPayload {
  const ShortcutPayload_LockScreen(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is ShortcutPayload_LockScreen);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'ShortcutPayload.lockScreen()';
}


}




/// @nodoc


class ShortcutPayload_Notifications extends ShortcutPayload {
  const ShortcutPayload_Notifications(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is ShortcutPayload_Notifications);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'ShortcutPayload.notifications()';
}


}




/// @nodoc


class ShortcutPayload_Android extends ShortcutPayload {
  const ShortcutPayload_Android({this.action, this.data, this.mimeType, required  List<String> categories, this.package, this.component, required this.flags, required  List<Extra> extras}): _categories = categories,_extras = extras,super._();
  

 final  String? action;
 final  String? data;
 final  String? mimeType;
 final  List<String> _categories;
 List<String> get categories {
  if (_categories is EqualUnmodifiableListView) return _categories;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_categories);
}

 final  String? package;
 final  String? component;
 final  int flags;
 final  List<Extra> _extras;
 List<Extra> get extras {
  if (_extras is EqualUnmodifiableListView) return _extras;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_extras);
}


/// Create a copy of ShortcutPayload
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ShortcutPayload_AndroidCopyWith<ShortcutPayload_Android> get copyWith => _$ShortcutPayload_AndroidCopyWithImpl<ShortcutPayload_Android>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is ShortcutPayload_Android&&(identical(other.action, action) || other.action == action)&&(identical(other.data, data) || other.data == data)&&(identical(other.mimeType, mimeType) || other.mimeType == mimeType)&&const DeepCollectionEquality().equals(other.categories, _categories)&&(identical(other.package, package) || other.package == package)&&(identical(other.component, component) || other.component == component)&&(identical(other.flags, flags) || other.flags == flags)&&const DeepCollectionEquality().equals(other.extras, _extras));
}


@override
int get hashCode {
    return Object.hash(runtimeType,action,data,mimeType,const DeepCollectionEquality().hash(_categories),package,component,flags,const DeepCollectionEquality().hash(_extras));
}

@override
String toString() {
    return 'ShortcutPayload.android(action: $action, data: $data, mimeType: $mimeType, categories: $categories, package: $package, component: $component, flags: $flags, extras: $extras)';
}


}

/// @nodoc
abstract mixin class $ShortcutPayload_AndroidCopyWith<$Res> implements $ShortcutPayloadCopyWith<$Res> {
  factory $ShortcutPayload_AndroidCopyWith(ShortcutPayload_Android value, $Res Function(ShortcutPayload_Android) _then) = _$ShortcutPayload_AndroidCopyWithImpl;
@useResult
$Res call({
 String? action, String? data, String? mimeType, List<String> categories, String? package, String? component, int flags, List<Extra> extras
});




}
/// @nodoc
class _$ShortcutPayload_AndroidCopyWithImpl<$Res>
    implements $ShortcutPayload_AndroidCopyWith<$Res> {
  _$ShortcutPayload_AndroidCopyWithImpl(this._self, this._then);

  final ShortcutPayload_Android _self;
  final $Res Function(ShortcutPayload_Android) _then;

/// Create a copy of ShortcutPayload
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? action = freezed,Object? data = freezed,Object? mimeType = freezed,Object? categories = null,Object? package = freezed,Object? component = freezed,Object? flags = null,Object? extras = null,}) {
  return _then(ShortcutPayload_Android(
action: freezed == action ? _self.action : action // ignore: cast_nullable_to_non_nullable
as String?,data: freezed == data ? _self.data : data // ignore: cast_nullable_to_non_nullable
as String?,mimeType: freezed == mimeType ? _self.mimeType : mimeType // ignore: cast_nullable_to_non_nullable
as String?,categories: null == categories ? _self._categories : categories // ignore: cast_nullable_to_non_nullable
as List<String>,package: freezed == package ? _self.package : package // ignore: cast_nullable_to_non_nullable
as String?,component: freezed == component ? _self.component : component // ignore: cast_nullable_to_non_nullable
as String?,flags: null == flags ? _self.flags : flags // ignore: cast_nullable_to_non_nullable
as int,extras: null == extras ? _self._extras : extras // ignore: cast_nullable_to_non_nullable
as List<Extra>,
  ));
}


}

/// @nodoc


class ShortcutPayload_Command extends ShortcutPayload {
  const ShortcutPayload_Command({required this.command}): super._();
  

 final  String command;

/// Create a copy of ShortcutPayload
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ShortcutPayload_CommandCopyWith<ShortcutPayload_Command> get copyWith => _$ShortcutPayload_CommandCopyWithImpl<ShortcutPayload_Command>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is ShortcutPayload_Command&&(identical(other.command, command) || other.command == command));
}


@override
int get hashCode {
    return Object.hash(runtimeType,command);
}

@override
String toString() {
    return 'ShortcutPayload.command(command: $command)';
}


}

/// @nodoc
abstract mixin class $ShortcutPayload_CommandCopyWith<$Res> implements $ShortcutPayloadCopyWith<$Res> {
  factory $ShortcutPayload_CommandCopyWith(ShortcutPayload_Command value, $Res Function(ShortcutPayload_Command) _then) = _$ShortcutPayload_CommandCopyWithImpl;
@useResult
$Res call({
 String command
});




}
/// @nodoc
class _$ShortcutPayload_CommandCopyWithImpl<$Res>
    implements $ShortcutPayload_CommandCopyWith<$Res> {
  _$ShortcutPayload_CommandCopyWithImpl(this._self, this._then);

  final ShortcutPayload_Command _self;
  final $Res Function(ShortcutPayload_Command) _then;

/// Create a copy of ShortcutPayload
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? command = null,}) {
  return _then(ShortcutPayload_Command(
command: null == command ? _self.command : command // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
