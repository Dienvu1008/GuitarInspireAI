// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'spike.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$CommandKind {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandKind);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CommandKind()';
}


}

/// @nodoc
class $CommandKindCopyWith<$Res>  {
$CommandKindCopyWith(CommandKind _, $Res Function(CommandKind) __);
}


/// Adds pattern-matching-related methods to [CommandKind].
extension CommandKindPatterns on CommandKind {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( CommandKind_Noop value)?  noop,TResult Function( CommandKind_InsertNote value)?  insertNote,TResult Function( CommandKind_SetTempo value)?  setTempo,required TResult orElse(),}){
final _that = this;
switch (_that) {
case CommandKind_Noop() when noop != null:
return noop(_that);case CommandKind_InsertNote() when insertNote != null:
return insertNote(_that);case CommandKind_SetTempo() when setTempo != null:
return setTempo(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( CommandKind_Noop value)  noop,required TResult Function( CommandKind_InsertNote value)  insertNote,required TResult Function( CommandKind_SetTempo value)  setTempo,}){
final _that = this;
switch (_that) {
case CommandKind_Noop():
return noop(_that);case CommandKind_InsertNote():
return insertNote(_that);case CommandKind_SetTempo():
return setTempo(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( CommandKind_Noop value)?  noop,TResult? Function( CommandKind_InsertNote value)?  insertNote,TResult? Function( CommandKind_SetTempo value)?  setTempo,}){
final _that = this;
switch (_that) {
case CommandKind_Noop() when noop != null:
return noop(_that);case CommandKind_InsertNote() when insertNote != null:
return insertNote(_that);case CommandKind_SetTempo() when setTempo != null:
return setTempo(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  noop,TResult Function( int stringIndex,  int fret)?  insertNote,TResult Function( int bpmNum,  int bpmDen)?  setTempo,required TResult orElse(),}) {final _that = this;
switch (_that) {
case CommandKind_Noop() when noop != null:
return noop();case CommandKind_InsertNote() when insertNote != null:
return insertNote(_that.stringIndex,_that.fret);case CommandKind_SetTempo() when setTempo != null:
return setTempo(_that.bpmNum,_that.bpmDen);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  noop,required TResult Function( int stringIndex,  int fret)  insertNote,required TResult Function( int bpmNum,  int bpmDen)  setTempo,}) {final _that = this;
switch (_that) {
case CommandKind_Noop():
return noop();case CommandKind_InsertNote():
return insertNote(_that.stringIndex,_that.fret);case CommandKind_SetTempo():
return setTempo(_that.bpmNum,_that.bpmDen);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  noop,TResult? Function( int stringIndex,  int fret)?  insertNote,TResult? Function( int bpmNum,  int bpmDen)?  setTempo,}) {final _that = this;
switch (_that) {
case CommandKind_Noop() when noop != null:
return noop();case CommandKind_InsertNote() when insertNote != null:
return insertNote(_that.stringIndex,_that.fret);case CommandKind_SetTempo() when setTempo != null:
return setTempo(_that.bpmNum,_that.bpmDen);case _:
  return null;

}
}

}

/// @nodoc


class CommandKind_Noop extends CommandKind {
  const CommandKind_Noop(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandKind_Noop);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CommandKind.noop()';
}


}




/// @nodoc


class CommandKind_InsertNote extends CommandKind {
  const CommandKind_InsertNote({required this.stringIndex, required this.fret}): super._();
  

 final  int stringIndex;
 final  int fret;

/// Create a copy of CommandKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandKind_InsertNoteCopyWith<CommandKind_InsertNote> get copyWith => _$CommandKind_InsertNoteCopyWithImpl<CommandKind_InsertNote>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandKind_InsertNote&&(identical(other.stringIndex, stringIndex) || other.stringIndex == stringIndex)&&(identical(other.fret, fret) || other.fret == fret));
}


@override
int get hashCode {
    return Object.hash(runtimeType,stringIndex,fret);
}

@override
String toString() {
    return 'CommandKind.insertNote(stringIndex: $stringIndex, fret: $fret)';
}


}

/// @nodoc
abstract mixin class $CommandKind_InsertNoteCopyWith<$Res> implements $CommandKindCopyWith<$Res> {
  factory $CommandKind_InsertNoteCopyWith(CommandKind_InsertNote value, $Res Function(CommandKind_InsertNote) _then) = _$CommandKind_InsertNoteCopyWithImpl;
@useResult
$Res call({
 int stringIndex, int fret
});




}
/// @nodoc
class _$CommandKind_InsertNoteCopyWithImpl<$Res>
    implements $CommandKind_InsertNoteCopyWith<$Res> {
  _$CommandKind_InsertNoteCopyWithImpl(this._self, this._then);

  final CommandKind_InsertNote _self;
  final $Res Function(CommandKind_InsertNote) _then;

/// Create a copy of CommandKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? stringIndex = null,Object? fret = null,}) {
  return _then(CommandKind_InsertNote(
stringIndex: null == stringIndex ? _self.stringIndex : stringIndex // ignore: cast_nullable_to_non_nullable
as int,fret: null == fret ? _self.fret : fret // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class CommandKind_SetTempo extends CommandKind {
  const CommandKind_SetTempo({required this.bpmNum, required this.bpmDen}): super._();
  

 final  int bpmNum;
 final  int bpmDen;

/// Create a copy of CommandKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandKind_SetTempoCopyWith<CommandKind_SetTempo> get copyWith => _$CommandKind_SetTempoCopyWithImpl<CommandKind_SetTempo>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandKind_SetTempo&&(identical(other.bpmNum, bpmNum) || other.bpmNum == bpmNum)&&(identical(other.bpmDen, bpmDen) || other.bpmDen == bpmDen));
}


@override
int get hashCode {
    return Object.hash(runtimeType,bpmNum,bpmDen);
}

@override
String toString() {
    return 'CommandKind.setTempo(bpmNum: $bpmNum, bpmDen: $bpmDen)';
}


}

/// @nodoc
abstract mixin class $CommandKind_SetTempoCopyWith<$Res> implements $CommandKindCopyWith<$Res> {
  factory $CommandKind_SetTempoCopyWith(CommandKind_SetTempo value, $Res Function(CommandKind_SetTempo) _then) = _$CommandKind_SetTempoCopyWithImpl;
@useResult
$Res call({
 int bpmNum, int bpmDen
});




}
/// @nodoc
class _$CommandKind_SetTempoCopyWithImpl<$Res>
    implements $CommandKind_SetTempoCopyWith<$Res> {
  _$CommandKind_SetTempoCopyWithImpl(this._self, this._then);

  final CommandKind_SetTempo _self;
  final $Res Function(CommandKind_SetTempo) _then;

/// Create a copy of CommandKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? bpmNum = null,Object? bpmDen = null,}) {
  return _then(CommandKind_SetTempo(
bpmNum: null == bpmNum ? _self.bpmNum : bpmNum // ignore: cast_nullable_to_non_nullable
as int,bpmDen: null == bpmDen ? _self.bpmDen : bpmDen // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

// dart format on
