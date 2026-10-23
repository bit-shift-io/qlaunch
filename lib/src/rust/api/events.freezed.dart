// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'events.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$CoreEvent {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CoreEvent()';
}


}

/// @nodoc
class $CoreEventCopyWith<$Res>  {
$CoreEventCopyWith(CoreEvent _, $Res Function(CoreEvent) __);
}


/// Adds pattern-matching-related methods to [CoreEvent].
extension CoreEventPatterns on CoreEvent {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( CoreEvent_ModelChanged value)?  modelChanged,TResult Function( CoreEvent_AppsChanged value)?  appsChanged,TResult Function( CoreEvent_ShortcutCreated value)?  shortcutCreated,TResult Function( CoreEvent_ShortcutCancelled value)?  shortcutCancelled,TResult Function( CoreEvent_WallpaperChanged value)?  wallpaperChanged,TResult Function( CoreEvent_HomePressed value)?  homePressed,TResult Function( CoreEvent_HomePressedFromOverlay value)?  homePressedFromOverlay,TResult Function( CoreEvent_BackPressed value)?  backPressed,TResult Function( CoreEvent_Error value)?  error,required TResult orElse(),}){
final _that = this;
switch (_that) {
case CoreEvent_ModelChanged() when modelChanged != null:
return modelChanged(_that);case CoreEvent_AppsChanged() when appsChanged != null:
return appsChanged(_that);case CoreEvent_ShortcutCreated() when shortcutCreated != null:
return shortcutCreated(_that);case CoreEvent_ShortcutCancelled() when shortcutCancelled != null:
return shortcutCancelled(_that);case CoreEvent_WallpaperChanged() when wallpaperChanged != null:
return wallpaperChanged(_that);case CoreEvent_HomePressed() when homePressed != null:
return homePressed(_that);case CoreEvent_HomePressedFromOverlay() when homePressedFromOverlay != null:
return homePressedFromOverlay(_that);case CoreEvent_BackPressed() when backPressed != null:
return backPressed(_that);case CoreEvent_Error() when error != null:
return error(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( CoreEvent_ModelChanged value)  modelChanged,required TResult Function( CoreEvent_AppsChanged value)  appsChanged,required TResult Function( CoreEvent_ShortcutCreated value)  shortcutCreated,required TResult Function( CoreEvent_ShortcutCancelled value)  shortcutCancelled,required TResult Function( CoreEvent_WallpaperChanged value)  wallpaperChanged,required TResult Function( CoreEvent_HomePressed value)  homePressed,required TResult Function( CoreEvent_HomePressedFromOverlay value)  homePressedFromOverlay,required TResult Function( CoreEvent_BackPressed value)  backPressed,required TResult Function( CoreEvent_Error value)  error,}){
final _that = this;
switch (_that) {
case CoreEvent_ModelChanged():
return modelChanged(_that);case CoreEvent_AppsChanged():
return appsChanged(_that);case CoreEvent_ShortcutCreated():
return shortcutCreated(_that);case CoreEvent_ShortcutCancelled():
return shortcutCancelled(_that);case CoreEvent_WallpaperChanged():
return wallpaperChanged(_that);case CoreEvent_HomePressed():
return homePressed(_that);case CoreEvent_HomePressedFromOverlay():
return homePressedFromOverlay(_that);case CoreEvent_BackPressed():
return backPressed(_that);case CoreEvent_Error():
return error(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( CoreEvent_ModelChanged value)?  modelChanged,TResult? Function( CoreEvent_AppsChanged value)?  appsChanged,TResult? Function( CoreEvent_ShortcutCreated value)?  shortcutCreated,TResult? Function( CoreEvent_ShortcutCancelled value)?  shortcutCancelled,TResult? Function( CoreEvent_WallpaperChanged value)?  wallpaperChanged,TResult? Function( CoreEvent_HomePressed value)?  homePressed,TResult? Function( CoreEvent_HomePressedFromOverlay value)?  homePressedFromOverlay,TResult? Function( CoreEvent_BackPressed value)?  backPressed,TResult? Function( CoreEvent_Error value)?  error,}){
final _that = this;
switch (_that) {
case CoreEvent_ModelChanged() when modelChanged != null:
return modelChanged(_that);case CoreEvent_AppsChanged() when appsChanged != null:
return appsChanged(_that);case CoreEvent_ShortcutCreated() when shortcutCreated != null:
return shortcutCreated(_that);case CoreEvent_ShortcutCancelled() when shortcutCancelled != null:
return shortcutCancelled(_that);case CoreEvent_WallpaperChanged() when wallpaperChanged != null:
return wallpaperChanged(_that);case CoreEvent_HomePressed() when homePressed != null:
return homePressed(_that);case CoreEvent_HomePressedFromOverlay() when homePressedFromOverlay != null:
return homePressedFromOverlay(_that);case CoreEvent_BackPressed() when backPressed != null:
return backPressed(_that);case CoreEvent_Error() when error != null:
return error(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  modelChanged,TResult Function()?  appsChanged,TResult Function( String itemId)?  shortcutCreated,TResult Function()?  shortcutCancelled,TResult Function()?  wallpaperChanged,TResult Function()?  homePressed,TResult Function()?  homePressedFromOverlay,TResult Function()?  backPressed,TResult Function( String field0)?  error,required TResult orElse(),}) {final _that = this;
switch (_that) {
case CoreEvent_ModelChanged() when modelChanged != null:
return modelChanged();case CoreEvent_AppsChanged() when appsChanged != null:
return appsChanged();case CoreEvent_ShortcutCreated() when shortcutCreated != null:
return shortcutCreated(_that.itemId);case CoreEvent_ShortcutCancelled() when shortcutCancelled != null:
return shortcutCancelled();case CoreEvent_WallpaperChanged() when wallpaperChanged != null:
return wallpaperChanged();case CoreEvent_HomePressed() when homePressed != null:
return homePressed();case CoreEvent_HomePressedFromOverlay() when homePressedFromOverlay != null:
return homePressedFromOverlay();case CoreEvent_BackPressed() when backPressed != null:
return backPressed();case CoreEvent_Error() when error != null:
return error(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  modelChanged,required TResult Function()  appsChanged,required TResult Function( String itemId)  shortcutCreated,required TResult Function()  shortcutCancelled,required TResult Function()  wallpaperChanged,required TResult Function()  homePressed,required TResult Function()  homePressedFromOverlay,required TResult Function()  backPressed,required TResult Function( String field0)  error,}) {final _that = this;
switch (_that) {
case CoreEvent_ModelChanged():
return modelChanged();case CoreEvent_AppsChanged():
return appsChanged();case CoreEvent_ShortcutCreated():
return shortcutCreated(_that.itemId);case CoreEvent_ShortcutCancelled():
return shortcutCancelled();case CoreEvent_WallpaperChanged():
return wallpaperChanged();case CoreEvent_HomePressed():
return homePressed();case CoreEvent_HomePressedFromOverlay():
return homePressedFromOverlay();case CoreEvent_BackPressed():
return backPressed();case CoreEvent_Error():
return error(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  modelChanged,TResult? Function()?  appsChanged,TResult? Function( String itemId)?  shortcutCreated,TResult? Function()?  shortcutCancelled,TResult? Function()?  wallpaperChanged,TResult? Function()?  homePressed,TResult? Function()?  homePressedFromOverlay,TResult? Function()?  backPressed,TResult? Function( String field0)?  error,}) {final _that = this;
switch (_that) {
case CoreEvent_ModelChanged() when modelChanged != null:
return modelChanged();case CoreEvent_AppsChanged() when appsChanged != null:
return appsChanged();case CoreEvent_ShortcutCreated() when shortcutCreated != null:
return shortcutCreated(_that.itemId);case CoreEvent_ShortcutCancelled() when shortcutCancelled != null:
return shortcutCancelled();case CoreEvent_WallpaperChanged() when wallpaperChanged != null:
return wallpaperChanged();case CoreEvent_HomePressed() when homePressed != null:
return homePressed();case CoreEvent_HomePressedFromOverlay() when homePressedFromOverlay != null:
return homePressedFromOverlay();case CoreEvent_BackPressed() when backPressed != null:
return backPressed();case CoreEvent_Error() when error != null:
return error(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class CoreEvent_ModelChanged extends CoreEvent {
  const CoreEvent_ModelChanged(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_ModelChanged);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CoreEvent.modelChanged()';
}


}




/// @nodoc


class CoreEvent_AppsChanged extends CoreEvent {
  const CoreEvent_AppsChanged(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_AppsChanged);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CoreEvent.appsChanged()';
}


}




/// @nodoc


class CoreEvent_ShortcutCreated extends CoreEvent {
  const CoreEvent_ShortcutCreated({required this.itemId}): super._();
  

 final  String itemId;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CoreEvent_ShortcutCreatedCopyWith<CoreEvent_ShortcutCreated> get copyWith => _$CoreEvent_ShortcutCreatedCopyWithImpl<CoreEvent_ShortcutCreated>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_ShortcutCreated&&(identical(other.itemId, itemId) || other.itemId == itemId));
}


@override
int get hashCode {
    return Object.hash(runtimeType,itemId);
}

@override
String toString() {
    return 'CoreEvent.shortcutCreated(itemId: $itemId)';
}


}

/// @nodoc
abstract mixin class $CoreEvent_ShortcutCreatedCopyWith<$Res> implements $CoreEventCopyWith<$Res> {
  factory $CoreEvent_ShortcutCreatedCopyWith(CoreEvent_ShortcutCreated value, $Res Function(CoreEvent_ShortcutCreated) _then) = _$CoreEvent_ShortcutCreatedCopyWithImpl;
@useResult
$Res call({
 String itemId
});




}
/// @nodoc
class _$CoreEvent_ShortcutCreatedCopyWithImpl<$Res>
    implements $CoreEvent_ShortcutCreatedCopyWith<$Res> {
  _$CoreEvent_ShortcutCreatedCopyWithImpl(this._self, this._then);

  final CoreEvent_ShortcutCreated _self;
  final $Res Function(CoreEvent_ShortcutCreated) _then;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? itemId = null,}) {
  return _then(CoreEvent_ShortcutCreated(
itemId: null == itemId ? _self.itemId : itemId // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class CoreEvent_ShortcutCancelled extends CoreEvent {
  const CoreEvent_ShortcutCancelled(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_ShortcutCancelled);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CoreEvent.shortcutCancelled()';
}


}




/// @nodoc


class CoreEvent_WallpaperChanged extends CoreEvent {
  const CoreEvent_WallpaperChanged(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_WallpaperChanged);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CoreEvent.wallpaperChanged()';
}


}




/// @nodoc


class CoreEvent_HomePressed extends CoreEvent {
  const CoreEvent_HomePressed(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_HomePressed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CoreEvent.homePressed()';
}


}




/// @nodoc


class CoreEvent_HomePressedFromOverlay extends CoreEvent {
  const CoreEvent_HomePressedFromOverlay(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_HomePressedFromOverlay);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CoreEvent.homePressedFromOverlay()';
}


}




/// @nodoc


class CoreEvent_BackPressed extends CoreEvent {
  const CoreEvent_BackPressed(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_BackPressed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CoreEvent.backPressed()';
}


}




/// @nodoc


class CoreEvent_Error extends CoreEvent {
  const CoreEvent_Error(this.field0): super._();
  

 final  String field0;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CoreEvent_ErrorCopyWith<CoreEvent_Error> get copyWith => _$CoreEvent_ErrorCopyWithImpl<CoreEvent_Error>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_Error&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'CoreEvent.error(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $CoreEvent_ErrorCopyWith<$Res> implements $CoreEventCopyWith<$Res> {
  factory $CoreEvent_ErrorCopyWith(CoreEvent_Error value, $Res Function(CoreEvent_Error) _then) = _$CoreEvent_ErrorCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$CoreEvent_ErrorCopyWithImpl<$Res>
    implements $CoreEvent_ErrorCopyWith<$Res> {
  _$CoreEvent_ErrorCopyWithImpl(this._self, this._then);

  final CoreEvent_Error _self;
  final $Res Function(CoreEvent_Error) _then;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(CoreEvent_Error(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
