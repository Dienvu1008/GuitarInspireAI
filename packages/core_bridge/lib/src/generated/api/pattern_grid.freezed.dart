// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'pattern_grid.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$GridActionDto {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridActionDto);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'GridActionDto()';
}


}

/// @nodoc
class $GridActionDtoCopyWith<$Res>  {
$GridActionDtoCopyWith(GridActionDto _, $Res Function(GridActionDto) __);
}


/// Adds pattern-matching-related methods to [GridActionDto].
extension GridActionDtoPatterns on GridActionDto {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( GridActionDto_Strike value)?  strike,TResult Function( GridActionDto_Gesture value)?  gesture,TResult Function( GridActionDto_Hold value)?  hold,TResult Function( GridActionDto_Rest value)?  rest,required TResult orElse(),}){
final _that = this;
switch (_that) {
case GridActionDto_Strike() when strike != null:
return strike(_that);case GridActionDto_Gesture() when gesture != null:
return gesture(_that);case GridActionDto_Hold() when hold != null:
return hold(_that);case GridActionDto_Rest() when rest != null:
return rest(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( GridActionDto_Strike value)  strike,required TResult Function( GridActionDto_Gesture value)  gesture,required TResult Function( GridActionDto_Hold value)  hold,required TResult Function( GridActionDto_Rest value)  rest,}){
final _that = this;
switch (_that) {
case GridActionDto_Strike():
return strike(_that);case GridActionDto_Gesture():
return gesture(_that);case GridActionDto_Hold():
return hold(_that);case GridActionDto_Rest():
return rest(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( GridActionDto_Strike value)?  strike,TResult? Function( GridActionDto_Gesture value)?  gesture,TResult? Function( GridActionDto_Hold value)?  hold,TResult? Function( GridActionDto_Rest value)?  rest,}){
final _that = this;
switch (_that) {
case GridActionDto_Strike() when strike != null:
return strike(_that);case GridActionDto_Gesture() when gesture != null:
return gesture(_that);case GridActionDto_Hold() when hold != null:
return hold(_that);case GridActionDto_Rest() when rest != null:
return rest(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( GridStrikeDto strike)?  strike,TResult Function( PlatformInt64 gesture)?  gesture,TResult Function( List<GridSelectorDto>? strings)?  hold,TResult Function()?  rest,required TResult orElse(),}) {final _that = this;
switch (_that) {
case GridActionDto_Strike() when strike != null:
return strike(_that.strike);case GridActionDto_Gesture() when gesture != null:
return gesture(_that.gesture);case GridActionDto_Hold() when hold != null:
return hold(_that.strings);case GridActionDto_Rest() when rest != null:
return rest();case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( GridStrikeDto strike)  strike,required TResult Function( PlatformInt64 gesture)  gesture,required TResult Function( List<GridSelectorDto>? strings)  hold,required TResult Function()  rest,}) {final _that = this;
switch (_that) {
case GridActionDto_Strike():
return strike(_that.strike);case GridActionDto_Gesture():
return gesture(_that.gesture);case GridActionDto_Hold():
return hold(_that.strings);case GridActionDto_Rest():
return rest();}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( GridStrikeDto strike)?  strike,TResult? Function( PlatformInt64 gesture)?  gesture,TResult? Function( List<GridSelectorDto>? strings)?  hold,TResult? Function()?  rest,}) {final _that = this;
switch (_that) {
case GridActionDto_Strike() when strike != null:
return strike(_that.strike);case GridActionDto_Gesture() when gesture != null:
return gesture(_that.gesture);case GridActionDto_Hold() when hold != null:
return hold(_that.strings);case GridActionDto_Rest() when rest != null:
return rest();case _:
  return null;

}
}

}

/// @nodoc


class GridActionDto_Strike extends GridActionDto {
  const GridActionDto_Strike({required this.strike}): super._();
  

 final  GridStrikeDto strike;

/// Create a copy of GridActionDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridActionDto_StrikeCopyWith<GridActionDto_Strike> get copyWith => _$GridActionDto_StrikeCopyWithImpl<GridActionDto_Strike>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridActionDto_Strike&&(identical(other.strike, strike) || other.strike == strike));
}


@override
int get hashCode {
    return Object.hash(runtimeType,strike);
}

@override
String toString() {
    return 'GridActionDto.strike(strike: $strike)';
}


}

/// @nodoc
abstract mixin class $GridActionDto_StrikeCopyWith<$Res> implements $GridActionDtoCopyWith<$Res> {
  factory $GridActionDto_StrikeCopyWith(GridActionDto_Strike value, $Res Function(GridActionDto_Strike) _then) = _$GridActionDto_StrikeCopyWithImpl;
@useResult
$Res call({
 GridStrikeDto strike
});




}
/// @nodoc
class _$GridActionDto_StrikeCopyWithImpl<$Res>
    implements $GridActionDto_StrikeCopyWith<$Res> {
  _$GridActionDto_StrikeCopyWithImpl(this._self, this._then);

  final GridActionDto_Strike _self;
  final $Res Function(GridActionDto_Strike) _then;

/// Create a copy of GridActionDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? strike = null,}) {
  return _then(GridActionDto_Strike(
strike: null == strike ? _self.strike : strike // ignore: cast_nullable_to_non_nullable
as GridStrikeDto,
  ));
}


}

/// @nodoc


class GridActionDto_Gesture extends GridActionDto {
  const GridActionDto_Gesture({required this.gesture}): super._();
  

 final  PlatformInt64 gesture;

/// Create a copy of GridActionDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridActionDto_GestureCopyWith<GridActionDto_Gesture> get copyWith => _$GridActionDto_GestureCopyWithImpl<GridActionDto_Gesture>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridActionDto_Gesture&&(identical(other.gesture, gesture) || other.gesture == gesture));
}


@override
int get hashCode {
    return Object.hash(runtimeType,gesture);
}

@override
String toString() {
    return 'GridActionDto.gesture(gesture: $gesture)';
}


}

/// @nodoc
abstract mixin class $GridActionDto_GestureCopyWith<$Res> implements $GridActionDtoCopyWith<$Res> {
  factory $GridActionDto_GestureCopyWith(GridActionDto_Gesture value, $Res Function(GridActionDto_Gesture) _then) = _$GridActionDto_GestureCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 gesture
});




}
/// @nodoc
class _$GridActionDto_GestureCopyWithImpl<$Res>
    implements $GridActionDto_GestureCopyWith<$Res> {
  _$GridActionDto_GestureCopyWithImpl(this._self, this._then);

  final GridActionDto_Gesture _self;
  final $Res Function(GridActionDto_Gesture) _then;

/// Create a copy of GridActionDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? gesture = null,}) {
  return _then(GridActionDto_Gesture(
gesture: null == gesture ? _self.gesture : gesture // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class GridActionDto_Hold extends GridActionDto {
  const GridActionDto_Hold({ List<GridSelectorDto>? strings}): _strings = strings,super._();
  

 final  List<GridSelectorDto>? _strings;
 List<GridSelectorDto>? get strings {
  final value = _strings;
  if (value == null) return null;
  if (_strings is EqualUnmodifiableListView) return _strings;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(value);
}


/// Create a copy of GridActionDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridActionDto_HoldCopyWith<GridActionDto_Hold> get copyWith => _$GridActionDto_HoldCopyWithImpl<GridActionDto_Hold>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridActionDto_Hold&&const DeepCollectionEquality().equals(other.strings, _strings));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_strings));
}

@override
String toString() {
    return 'GridActionDto.hold(strings: $strings)';
}


}

/// @nodoc
abstract mixin class $GridActionDto_HoldCopyWith<$Res> implements $GridActionDtoCopyWith<$Res> {
  factory $GridActionDto_HoldCopyWith(GridActionDto_Hold value, $Res Function(GridActionDto_Hold) _then) = _$GridActionDto_HoldCopyWithImpl;
@useResult
$Res call({
 List<GridSelectorDto>? strings
});




}
/// @nodoc
class _$GridActionDto_HoldCopyWithImpl<$Res>
    implements $GridActionDto_HoldCopyWith<$Res> {
  _$GridActionDto_HoldCopyWithImpl(this._self, this._then);

  final GridActionDto_Hold _self;
  final $Res Function(GridActionDto_Hold) _then;

/// Create a copy of GridActionDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? strings = freezed,}) {
  return _then(GridActionDto_Hold(
strings: freezed == strings ? _self._strings : strings // ignore: cast_nullable_to_non_nullable
as List<GridSelectorDto>?,
  ));
}


}

/// @nodoc


class GridActionDto_Rest extends GridActionDto {
  const GridActionDto_Rest(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridActionDto_Rest);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'GridActionDto.rest()';
}


}




/// @nodoc
mixin _$GridEditDto {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'GridEditDto()';
}


}

/// @nodoc
class $GridEditDtoCopyWith<$Res>  {
$GridEditDtoCopyWith(GridEditDto _, $Res Function(GridEditDto) __);
}


/// Adds pattern-matching-related methods to [GridEditDto].
extension GridEditDtoPatterns on GridEditDto {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( GridEditDto_Place value)?  place,TResult Function( GridEditDto_Move value)?  move,TResult Function( GridEditDto_Resize value)?  resize,TResult Function( GridEditDto_Delete value)?  delete,TResult Function( GridEditDto_SetAction value)?  setAction,TResult Function( GridEditDto_ToggleTarget value)?  toggleTarget,TResult Function( GridEditDto_SetDirection value)?  setDirection,TResult Function( GridEditDto_SetTechnique value)?  setTechnique,TResult Function( GridEditDto_AddSpan value)?  addSpan,TResult Function( GridEditDto_RemoveSpan value)?  removeSpan,TResult Function( GridEditDto_SetLength value)?  setLength,TResult Function( GridEditDto_SetMeter value)?  setMeter,TResult Function( GridEditDto_SetName value)?  setName,TResult Function( GridEditDto_SetMeta value)?  setMeta,required TResult orElse(),}){
final _that = this;
switch (_that) {
case GridEditDto_Place() when place != null:
return place(_that);case GridEditDto_Move() when move != null:
return move(_that);case GridEditDto_Resize() when resize != null:
return resize(_that);case GridEditDto_Delete() when delete != null:
return delete(_that);case GridEditDto_SetAction() when setAction != null:
return setAction(_that);case GridEditDto_ToggleTarget() when toggleTarget != null:
return toggleTarget(_that);case GridEditDto_SetDirection() when setDirection != null:
return setDirection(_that);case GridEditDto_SetTechnique() when setTechnique != null:
return setTechnique(_that);case GridEditDto_AddSpan() when addSpan != null:
return addSpan(_that);case GridEditDto_RemoveSpan() when removeSpan != null:
return removeSpan(_that);case GridEditDto_SetLength() when setLength != null:
return setLength(_that);case GridEditDto_SetMeter() when setMeter != null:
return setMeter(_that);case GridEditDto_SetName() when setName != null:
return setName(_that);case GridEditDto_SetMeta() when setMeta != null:
return setMeta(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( GridEditDto_Place value)  place,required TResult Function( GridEditDto_Move value)  move,required TResult Function( GridEditDto_Resize value)  resize,required TResult Function( GridEditDto_Delete value)  delete,required TResult Function( GridEditDto_SetAction value)  setAction,required TResult Function( GridEditDto_ToggleTarget value)  toggleTarget,required TResult Function( GridEditDto_SetDirection value)  setDirection,required TResult Function( GridEditDto_SetTechnique value)  setTechnique,required TResult Function( GridEditDto_AddSpan value)  addSpan,required TResult Function( GridEditDto_RemoveSpan value)  removeSpan,required TResult Function( GridEditDto_SetLength value)  setLength,required TResult Function( GridEditDto_SetMeter value)  setMeter,required TResult Function( GridEditDto_SetName value)  setName,required TResult Function( GridEditDto_SetMeta value)  setMeta,}){
final _that = this;
switch (_that) {
case GridEditDto_Place():
return place(_that);case GridEditDto_Move():
return move(_that);case GridEditDto_Resize():
return resize(_that);case GridEditDto_Delete():
return delete(_that);case GridEditDto_SetAction():
return setAction(_that);case GridEditDto_ToggleTarget():
return toggleTarget(_that);case GridEditDto_SetDirection():
return setDirection(_that);case GridEditDto_SetTechnique():
return setTechnique(_that);case GridEditDto_AddSpan():
return addSpan(_that);case GridEditDto_RemoveSpan():
return removeSpan(_that);case GridEditDto_SetLength():
return setLength(_that);case GridEditDto_SetMeter():
return setMeter(_that);case GridEditDto_SetName():
return setName(_that);case GridEditDto_SetMeta():
return setMeta(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( GridEditDto_Place value)?  place,TResult? Function( GridEditDto_Move value)?  move,TResult? Function( GridEditDto_Resize value)?  resize,TResult? Function( GridEditDto_Delete value)?  delete,TResult? Function( GridEditDto_SetAction value)?  setAction,TResult? Function( GridEditDto_ToggleTarget value)?  toggleTarget,TResult? Function( GridEditDto_SetDirection value)?  setDirection,TResult? Function( GridEditDto_SetTechnique value)?  setTechnique,TResult? Function( GridEditDto_AddSpan value)?  addSpan,TResult? Function( GridEditDto_RemoveSpan value)?  removeSpan,TResult? Function( GridEditDto_SetLength value)?  setLength,TResult? Function( GridEditDto_SetMeter value)?  setMeter,TResult? Function( GridEditDto_SetName value)?  setName,TResult? Function( GridEditDto_SetMeta value)?  setMeta,}){
final _that = this;
switch (_that) {
case GridEditDto_Place() when place != null:
return place(_that);case GridEditDto_Move() when move != null:
return move(_that);case GridEditDto_Resize() when resize != null:
return resize(_that);case GridEditDto_Delete() when delete != null:
return delete(_that);case GridEditDto_SetAction() when setAction != null:
return setAction(_that);case GridEditDto_ToggleTarget() when toggleTarget != null:
return toggleTarget(_that);case GridEditDto_SetDirection() when setDirection != null:
return setDirection(_that);case GridEditDto_SetTechnique() when setTechnique != null:
return setTechnique(_that);case GridEditDto_AddSpan() when addSpan != null:
return addSpan(_that);case GridEditDto_RemoveSpan() when removeSpan != null:
return removeSpan(_that);case GridEditDto_SetLength() when setLength != null:
return setLength(_that);case GridEditDto_SetMeter() when setMeter != null:
return setMeter(_that);case GridEditDto_SetName() when setName != null:
return setName(_that);case GridEditDto_SetMeta() when setMeta != null:
return setMeta(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( PatternLaneDto lane,  GridCellDto at,  int cells,  GridActionDto action)?  place,TResult Function( PatternLaneDto lane,  int index,  GridCellDto to)?  move,TResult Function( PatternLaneDto lane,  int index,  int parts,  int cells)?  resize,TResult Function( PatternLaneDto lane,  int first,  int last)?  delete,TResult Function( PatternLaneDto lane,  int index,  GridActionDto action)?  setAction,TResult Function( PatternLaneDto lane,  int index,  GridSelectorDto selector)?  toggleTarget,TResult Function( PatternLaneDto lane,  int first,  int last,  StrokeDto? direction)?  setDirection,TResult Function( PatternLaneDto lane,  int index,  TechniqueKindDto technique,  bool on_)?  setTechnique,TResult Function( PatternLaneDto lane,  int first,  int last,  TechniqueKindDto technique)?  addSpan,TResult Function( int index)?  removeSpan,TResult Function( PlatformInt64 numerator,  PlatformInt64 denominator)?  setLength,TResult Function( MeterDto? meter)?  setMeter,TResult Function( String name)?  setName,TResult Function( String description,  List<String> tags)?  setMeta,required TResult orElse(),}) {final _that = this;
switch (_that) {
case GridEditDto_Place() when place != null:
return place(_that.lane,_that.at,_that.cells,_that.action);case GridEditDto_Move() when move != null:
return move(_that.lane,_that.index,_that.to);case GridEditDto_Resize() when resize != null:
return resize(_that.lane,_that.index,_that.parts,_that.cells);case GridEditDto_Delete() when delete != null:
return delete(_that.lane,_that.first,_that.last);case GridEditDto_SetAction() when setAction != null:
return setAction(_that.lane,_that.index,_that.action);case GridEditDto_ToggleTarget() when toggleTarget != null:
return toggleTarget(_that.lane,_that.index,_that.selector);case GridEditDto_SetDirection() when setDirection != null:
return setDirection(_that.lane,_that.first,_that.last,_that.direction);case GridEditDto_SetTechnique() when setTechnique != null:
return setTechnique(_that.lane,_that.index,_that.technique,_that.on_);case GridEditDto_AddSpan() when addSpan != null:
return addSpan(_that.lane,_that.first,_that.last,_that.technique);case GridEditDto_RemoveSpan() when removeSpan != null:
return removeSpan(_that.index);case GridEditDto_SetLength() when setLength != null:
return setLength(_that.numerator,_that.denominator);case GridEditDto_SetMeter() when setMeter != null:
return setMeter(_that.meter);case GridEditDto_SetName() when setName != null:
return setName(_that.name);case GridEditDto_SetMeta() when setMeta != null:
return setMeta(_that.description,_that.tags);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( PatternLaneDto lane,  GridCellDto at,  int cells,  GridActionDto action)  place,required TResult Function( PatternLaneDto lane,  int index,  GridCellDto to)  move,required TResult Function( PatternLaneDto lane,  int index,  int parts,  int cells)  resize,required TResult Function( PatternLaneDto lane,  int first,  int last)  delete,required TResult Function( PatternLaneDto lane,  int index,  GridActionDto action)  setAction,required TResult Function( PatternLaneDto lane,  int index,  GridSelectorDto selector)  toggleTarget,required TResult Function( PatternLaneDto lane,  int first,  int last,  StrokeDto? direction)  setDirection,required TResult Function( PatternLaneDto lane,  int index,  TechniqueKindDto technique,  bool on_)  setTechnique,required TResult Function( PatternLaneDto lane,  int first,  int last,  TechniqueKindDto technique)  addSpan,required TResult Function( int index)  removeSpan,required TResult Function( PlatformInt64 numerator,  PlatformInt64 denominator)  setLength,required TResult Function( MeterDto? meter)  setMeter,required TResult Function( String name)  setName,required TResult Function( String description,  List<String> tags)  setMeta,}) {final _that = this;
switch (_that) {
case GridEditDto_Place():
return place(_that.lane,_that.at,_that.cells,_that.action);case GridEditDto_Move():
return move(_that.lane,_that.index,_that.to);case GridEditDto_Resize():
return resize(_that.lane,_that.index,_that.parts,_that.cells);case GridEditDto_Delete():
return delete(_that.lane,_that.first,_that.last);case GridEditDto_SetAction():
return setAction(_that.lane,_that.index,_that.action);case GridEditDto_ToggleTarget():
return toggleTarget(_that.lane,_that.index,_that.selector);case GridEditDto_SetDirection():
return setDirection(_that.lane,_that.first,_that.last,_that.direction);case GridEditDto_SetTechnique():
return setTechnique(_that.lane,_that.index,_that.technique,_that.on_);case GridEditDto_AddSpan():
return addSpan(_that.lane,_that.first,_that.last,_that.technique);case GridEditDto_RemoveSpan():
return removeSpan(_that.index);case GridEditDto_SetLength():
return setLength(_that.numerator,_that.denominator);case GridEditDto_SetMeter():
return setMeter(_that.meter);case GridEditDto_SetName():
return setName(_that.name);case GridEditDto_SetMeta():
return setMeta(_that.description,_that.tags);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( PatternLaneDto lane,  GridCellDto at,  int cells,  GridActionDto action)?  place,TResult? Function( PatternLaneDto lane,  int index,  GridCellDto to)?  move,TResult? Function( PatternLaneDto lane,  int index,  int parts,  int cells)?  resize,TResult? Function( PatternLaneDto lane,  int first,  int last)?  delete,TResult? Function( PatternLaneDto lane,  int index,  GridActionDto action)?  setAction,TResult? Function( PatternLaneDto lane,  int index,  GridSelectorDto selector)?  toggleTarget,TResult? Function( PatternLaneDto lane,  int first,  int last,  StrokeDto? direction)?  setDirection,TResult? Function( PatternLaneDto lane,  int index,  TechniqueKindDto technique,  bool on_)?  setTechnique,TResult? Function( PatternLaneDto lane,  int first,  int last,  TechniqueKindDto technique)?  addSpan,TResult? Function( int index)?  removeSpan,TResult? Function( PlatformInt64 numerator,  PlatformInt64 denominator)?  setLength,TResult? Function( MeterDto? meter)?  setMeter,TResult? Function( String name)?  setName,TResult? Function( String description,  List<String> tags)?  setMeta,}) {final _that = this;
switch (_that) {
case GridEditDto_Place() when place != null:
return place(_that.lane,_that.at,_that.cells,_that.action);case GridEditDto_Move() when move != null:
return move(_that.lane,_that.index,_that.to);case GridEditDto_Resize() when resize != null:
return resize(_that.lane,_that.index,_that.parts,_that.cells);case GridEditDto_Delete() when delete != null:
return delete(_that.lane,_that.first,_that.last);case GridEditDto_SetAction() when setAction != null:
return setAction(_that.lane,_that.index,_that.action);case GridEditDto_ToggleTarget() when toggleTarget != null:
return toggleTarget(_that.lane,_that.index,_that.selector);case GridEditDto_SetDirection() when setDirection != null:
return setDirection(_that.lane,_that.first,_that.last,_that.direction);case GridEditDto_SetTechnique() when setTechnique != null:
return setTechnique(_that.lane,_that.index,_that.technique,_that.on_);case GridEditDto_AddSpan() when addSpan != null:
return addSpan(_that.lane,_that.first,_that.last,_that.technique);case GridEditDto_RemoveSpan() when removeSpan != null:
return removeSpan(_that.index);case GridEditDto_SetLength() when setLength != null:
return setLength(_that.numerator,_that.denominator);case GridEditDto_SetMeter() when setMeter != null:
return setMeter(_that.meter);case GridEditDto_SetName() when setName != null:
return setName(_that.name);case GridEditDto_SetMeta() when setMeta != null:
return setMeta(_that.description,_that.tags);case _:
  return null;

}
}

}

/// @nodoc


class GridEditDto_Place extends GridEditDto {
  const GridEditDto_Place({required this.lane, required this.at, required this.cells, required this.action}): super._();
  

 final  PatternLaneDto lane;
 final  GridCellDto at;
 final  int cells;
 final  GridActionDto action;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_PlaceCopyWith<GridEditDto_Place> get copyWith => _$GridEditDto_PlaceCopyWithImpl<GridEditDto_Place>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_Place&&(identical(other.lane, lane) || other.lane == lane)&&(identical(other.at, at) || other.at == at)&&(identical(other.cells, cells) || other.cells == cells)&&(identical(other.action, action) || other.action == action));
}


@override
int get hashCode {
    return Object.hash(runtimeType,lane,at,cells,action);
}

@override
String toString() {
    return 'GridEditDto.place(lane: $lane, at: $at, cells: $cells, action: $action)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_PlaceCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_PlaceCopyWith(GridEditDto_Place value, $Res Function(GridEditDto_Place) _then) = _$GridEditDto_PlaceCopyWithImpl;
@useResult
$Res call({
 PatternLaneDto lane, GridCellDto at, int cells, GridActionDto action
});


$GridActionDtoCopyWith<$Res> get action;

}
/// @nodoc
class _$GridEditDto_PlaceCopyWithImpl<$Res>
    implements $GridEditDto_PlaceCopyWith<$Res> {
  _$GridEditDto_PlaceCopyWithImpl(this._self, this._then);

  final GridEditDto_Place _self;
  final $Res Function(GridEditDto_Place) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? lane = null,Object? at = null,Object? cells = null,Object? action = null,}) {
  return _then(GridEditDto_Place(
lane: null == lane ? _self.lane : lane // ignore: cast_nullable_to_non_nullable
as PatternLaneDto,at: null == at ? _self.at : at // ignore: cast_nullable_to_non_nullable
as GridCellDto,cells: null == cells ? _self.cells : cells // ignore: cast_nullable_to_non_nullable
as int,action: null == action ? _self.action : action // ignore: cast_nullable_to_non_nullable
as GridActionDto,
  ));
}

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$GridActionDtoCopyWith<$Res> get action {
  
  return $GridActionDtoCopyWith<$Res>(_self.action, (value) {
    return _then(_self.copyWith(action: value));
  });
}
}

/// @nodoc


class GridEditDto_Move extends GridEditDto {
  const GridEditDto_Move({required this.lane, required this.index, required this.to}): super._();
  

 final  PatternLaneDto lane;
 final  int index;
 final  GridCellDto to;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_MoveCopyWith<GridEditDto_Move> get copyWith => _$GridEditDto_MoveCopyWithImpl<GridEditDto_Move>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_Move&&(identical(other.lane, lane) || other.lane == lane)&&(identical(other.index, index) || other.index == index)&&(identical(other.to, to) || other.to == to));
}


@override
int get hashCode {
    return Object.hash(runtimeType,lane,index,to);
}

@override
String toString() {
    return 'GridEditDto.move(lane: $lane, index: $index, to: $to)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_MoveCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_MoveCopyWith(GridEditDto_Move value, $Res Function(GridEditDto_Move) _then) = _$GridEditDto_MoveCopyWithImpl;
@useResult
$Res call({
 PatternLaneDto lane, int index, GridCellDto to
});




}
/// @nodoc
class _$GridEditDto_MoveCopyWithImpl<$Res>
    implements $GridEditDto_MoveCopyWith<$Res> {
  _$GridEditDto_MoveCopyWithImpl(this._self, this._then);

  final GridEditDto_Move _self;
  final $Res Function(GridEditDto_Move) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? lane = null,Object? index = null,Object? to = null,}) {
  return _then(GridEditDto_Move(
lane: null == lane ? _self.lane : lane // ignore: cast_nullable_to_non_nullable
as PatternLaneDto,index: null == index ? _self.index : index // ignore: cast_nullable_to_non_nullable
as int,to: null == to ? _self.to : to // ignore: cast_nullable_to_non_nullable
as GridCellDto,
  ));
}


}

/// @nodoc


class GridEditDto_Resize extends GridEditDto {
  const GridEditDto_Resize({required this.lane, required this.index, required this.parts, required this.cells}): super._();
  

 final  PatternLaneDto lane;
 final  int index;
 final  int parts;
 final  int cells;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_ResizeCopyWith<GridEditDto_Resize> get copyWith => _$GridEditDto_ResizeCopyWithImpl<GridEditDto_Resize>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_Resize&&(identical(other.lane, lane) || other.lane == lane)&&(identical(other.index, index) || other.index == index)&&(identical(other.parts, parts) || other.parts == parts)&&(identical(other.cells, cells) || other.cells == cells));
}


@override
int get hashCode {
    return Object.hash(runtimeType,lane,index,parts,cells);
}

@override
String toString() {
    return 'GridEditDto.resize(lane: $lane, index: $index, parts: $parts, cells: $cells)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_ResizeCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_ResizeCopyWith(GridEditDto_Resize value, $Res Function(GridEditDto_Resize) _then) = _$GridEditDto_ResizeCopyWithImpl;
@useResult
$Res call({
 PatternLaneDto lane, int index, int parts, int cells
});




}
/// @nodoc
class _$GridEditDto_ResizeCopyWithImpl<$Res>
    implements $GridEditDto_ResizeCopyWith<$Res> {
  _$GridEditDto_ResizeCopyWithImpl(this._self, this._then);

  final GridEditDto_Resize _self;
  final $Res Function(GridEditDto_Resize) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? lane = null,Object? index = null,Object? parts = null,Object? cells = null,}) {
  return _then(GridEditDto_Resize(
lane: null == lane ? _self.lane : lane // ignore: cast_nullable_to_non_nullable
as PatternLaneDto,index: null == index ? _self.index : index // ignore: cast_nullable_to_non_nullable
as int,parts: null == parts ? _self.parts : parts // ignore: cast_nullable_to_non_nullable
as int,cells: null == cells ? _self.cells : cells // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class GridEditDto_Delete extends GridEditDto {
  const GridEditDto_Delete({required this.lane, required this.first, required this.last}): super._();
  

 final  PatternLaneDto lane;
 final  int first;
 final  int last;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_DeleteCopyWith<GridEditDto_Delete> get copyWith => _$GridEditDto_DeleteCopyWithImpl<GridEditDto_Delete>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_Delete&&(identical(other.lane, lane) || other.lane == lane)&&(identical(other.first, first) || other.first == first)&&(identical(other.last, last) || other.last == last));
}


@override
int get hashCode {
    return Object.hash(runtimeType,lane,first,last);
}

@override
String toString() {
    return 'GridEditDto.delete(lane: $lane, first: $first, last: $last)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_DeleteCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_DeleteCopyWith(GridEditDto_Delete value, $Res Function(GridEditDto_Delete) _then) = _$GridEditDto_DeleteCopyWithImpl;
@useResult
$Res call({
 PatternLaneDto lane, int first, int last
});




}
/// @nodoc
class _$GridEditDto_DeleteCopyWithImpl<$Res>
    implements $GridEditDto_DeleteCopyWith<$Res> {
  _$GridEditDto_DeleteCopyWithImpl(this._self, this._then);

  final GridEditDto_Delete _self;
  final $Res Function(GridEditDto_Delete) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? lane = null,Object? first = null,Object? last = null,}) {
  return _then(GridEditDto_Delete(
lane: null == lane ? _self.lane : lane // ignore: cast_nullable_to_non_nullable
as PatternLaneDto,first: null == first ? _self.first : first // ignore: cast_nullable_to_non_nullable
as int,last: null == last ? _self.last : last // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class GridEditDto_SetAction extends GridEditDto {
  const GridEditDto_SetAction({required this.lane, required this.index, required this.action}): super._();
  

 final  PatternLaneDto lane;
 final  int index;
 final  GridActionDto action;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_SetActionCopyWith<GridEditDto_SetAction> get copyWith => _$GridEditDto_SetActionCopyWithImpl<GridEditDto_SetAction>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_SetAction&&(identical(other.lane, lane) || other.lane == lane)&&(identical(other.index, index) || other.index == index)&&(identical(other.action, action) || other.action == action));
}


@override
int get hashCode {
    return Object.hash(runtimeType,lane,index,action);
}

@override
String toString() {
    return 'GridEditDto.setAction(lane: $lane, index: $index, action: $action)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_SetActionCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_SetActionCopyWith(GridEditDto_SetAction value, $Res Function(GridEditDto_SetAction) _then) = _$GridEditDto_SetActionCopyWithImpl;
@useResult
$Res call({
 PatternLaneDto lane, int index, GridActionDto action
});


$GridActionDtoCopyWith<$Res> get action;

}
/// @nodoc
class _$GridEditDto_SetActionCopyWithImpl<$Res>
    implements $GridEditDto_SetActionCopyWith<$Res> {
  _$GridEditDto_SetActionCopyWithImpl(this._self, this._then);

  final GridEditDto_SetAction _self;
  final $Res Function(GridEditDto_SetAction) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? lane = null,Object? index = null,Object? action = null,}) {
  return _then(GridEditDto_SetAction(
lane: null == lane ? _self.lane : lane // ignore: cast_nullable_to_non_nullable
as PatternLaneDto,index: null == index ? _self.index : index // ignore: cast_nullable_to_non_nullable
as int,action: null == action ? _self.action : action // ignore: cast_nullable_to_non_nullable
as GridActionDto,
  ));
}

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$GridActionDtoCopyWith<$Res> get action {
  
  return $GridActionDtoCopyWith<$Res>(_self.action, (value) {
    return _then(_self.copyWith(action: value));
  });
}
}

/// @nodoc


class GridEditDto_ToggleTarget extends GridEditDto {
  const GridEditDto_ToggleTarget({required this.lane, required this.index, required this.selector}): super._();
  

 final  PatternLaneDto lane;
 final  int index;
 final  GridSelectorDto selector;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_ToggleTargetCopyWith<GridEditDto_ToggleTarget> get copyWith => _$GridEditDto_ToggleTargetCopyWithImpl<GridEditDto_ToggleTarget>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_ToggleTarget&&(identical(other.lane, lane) || other.lane == lane)&&(identical(other.index, index) || other.index == index)&&(identical(other.selector, selector) || other.selector == selector));
}


@override
int get hashCode {
    return Object.hash(runtimeType,lane,index,selector);
}

@override
String toString() {
    return 'GridEditDto.toggleTarget(lane: $lane, index: $index, selector: $selector)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_ToggleTargetCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_ToggleTargetCopyWith(GridEditDto_ToggleTarget value, $Res Function(GridEditDto_ToggleTarget) _then) = _$GridEditDto_ToggleTargetCopyWithImpl;
@useResult
$Res call({
 PatternLaneDto lane, int index, GridSelectorDto selector
});




}
/// @nodoc
class _$GridEditDto_ToggleTargetCopyWithImpl<$Res>
    implements $GridEditDto_ToggleTargetCopyWith<$Res> {
  _$GridEditDto_ToggleTargetCopyWithImpl(this._self, this._then);

  final GridEditDto_ToggleTarget _self;
  final $Res Function(GridEditDto_ToggleTarget) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? lane = null,Object? index = null,Object? selector = null,}) {
  return _then(GridEditDto_ToggleTarget(
lane: null == lane ? _self.lane : lane // ignore: cast_nullable_to_non_nullable
as PatternLaneDto,index: null == index ? _self.index : index // ignore: cast_nullable_to_non_nullable
as int,selector: null == selector ? _self.selector : selector // ignore: cast_nullable_to_non_nullable
as GridSelectorDto,
  ));
}


}

/// @nodoc


class GridEditDto_SetDirection extends GridEditDto {
  const GridEditDto_SetDirection({required this.lane, required this.first, required this.last, this.direction}): super._();
  

 final  PatternLaneDto lane;
 final  int first;
 final  int last;
 final  StrokeDto? direction;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_SetDirectionCopyWith<GridEditDto_SetDirection> get copyWith => _$GridEditDto_SetDirectionCopyWithImpl<GridEditDto_SetDirection>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_SetDirection&&(identical(other.lane, lane) || other.lane == lane)&&(identical(other.first, first) || other.first == first)&&(identical(other.last, last) || other.last == last)&&(identical(other.direction, direction) || other.direction == direction));
}


@override
int get hashCode {
    return Object.hash(runtimeType,lane,first,last,direction);
}

@override
String toString() {
    return 'GridEditDto.setDirection(lane: $lane, first: $first, last: $last, direction: $direction)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_SetDirectionCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_SetDirectionCopyWith(GridEditDto_SetDirection value, $Res Function(GridEditDto_SetDirection) _then) = _$GridEditDto_SetDirectionCopyWithImpl;
@useResult
$Res call({
 PatternLaneDto lane, int first, int last, StrokeDto? direction
});




}
/// @nodoc
class _$GridEditDto_SetDirectionCopyWithImpl<$Res>
    implements $GridEditDto_SetDirectionCopyWith<$Res> {
  _$GridEditDto_SetDirectionCopyWithImpl(this._self, this._then);

  final GridEditDto_SetDirection _self;
  final $Res Function(GridEditDto_SetDirection) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? lane = null,Object? first = null,Object? last = null,Object? direction = freezed,}) {
  return _then(GridEditDto_SetDirection(
lane: null == lane ? _self.lane : lane // ignore: cast_nullable_to_non_nullable
as PatternLaneDto,first: null == first ? _self.first : first // ignore: cast_nullable_to_non_nullable
as int,last: null == last ? _self.last : last // ignore: cast_nullable_to_non_nullable
as int,direction: freezed == direction ? _self.direction : direction // ignore: cast_nullable_to_non_nullable
as StrokeDto?,
  ));
}


}

/// @nodoc


class GridEditDto_SetTechnique extends GridEditDto {
  const GridEditDto_SetTechnique({required this.lane, required this.index, required this.technique, required this.on_}): super._();
  

 final  PatternLaneDto lane;
 final  int index;
 final  TechniqueKindDto technique;
 final  bool on_;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_SetTechniqueCopyWith<GridEditDto_SetTechnique> get copyWith => _$GridEditDto_SetTechniqueCopyWithImpl<GridEditDto_SetTechnique>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_SetTechnique&&(identical(other.lane, lane) || other.lane == lane)&&(identical(other.index, index) || other.index == index)&&(identical(other.technique, technique) || other.technique == technique)&&(identical(other.on_, on_) || other.on_ == on_));
}


@override
int get hashCode {
    return Object.hash(runtimeType,lane,index,technique,on_);
}

@override
String toString() {
    return 'GridEditDto.setTechnique(lane: $lane, index: $index, technique: $technique, on_: $on_)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_SetTechniqueCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_SetTechniqueCopyWith(GridEditDto_SetTechnique value, $Res Function(GridEditDto_SetTechnique) _then) = _$GridEditDto_SetTechniqueCopyWithImpl;
@useResult
$Res call({
 PatternLaneDto lane, int index, TechniqueKindDto technique, bool on_
});




}
/// @nodoc
class _$GridEditDto_SetTechniqueCopyWithImpl<$Res>
    implements $GridEditDto_SetTechniqueCopyWith<$Res> {
  _$GridEditDto_SetTechniqueCopyWithImpl(this._self, this._then);

  final GridEditDto_SetTechnique _self;
  final $Res Function(GridEditDto_SetTechnique) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? lane = null,Object? index = null,Object? technique = null,Object? on_ = null,}) {
  return _then(GridEditDto_SetTechnique(
lane: null == lane ? _self.lane : lane // ignore: cast_nullable_to_non_nullable
as PatternLaneDto,index: null == index ? _self.index : index // ignore: cast_nullable_to_non_nullable
as int,technique: null == technique ? _self.technique : technique // ignore: cast_nullable_to_non_nullable
as TechniqueKindDto,on_: null == on_ ? _self.on_ : on_ // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class GridEditDto_AddSpan extends GridEditDto {
  const GridEditDto_AddSpan({required this.lane, required this.first, required this.last, required this.technique}): super._();
  

 final  PatternLaneDto lane;
 final  int first;
 final  int last;
 final  TechniqueKindDto technique;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_AddSpanCopyWith<GridEditDto_AddSpan> get copyWith => _$GridEditDto_AddSpanCopyWithImpl<GridEditDto_AddSpan>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_AddSpan&&(identical(other.lane, lane) || other.lane == lane)&&(identical(other.first, first) || other.first == first)&&(identical(other.last, last) || other.last == last)&&(identical(other.technique, technique) || other.technique == technique));
}


@override
int get hashCode {
    return Object.hash(runtimeType,lane,first,last,technique);
}

@override
String toString() {
    return 'GridEditDto.addSpan(lane: $lane, first: $first, last: $last, technique: $technique)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_AddSpanCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_AddSpanCopyWith(GridEditDto_AddSpan value, $Res Function(GridEditDto_AddSpan) _then) = _$GridEditDto_AddSpanCopyWithImpl;
@useResult
$Res call({
 PatternLaneDto lane, int first, int last, TechniqueKindDto technique
});




}
/// @nodoc
class _$GridEditDto_AddSpanCopyWithImpl<$Res>
    implements $GridEditDto_AddSpanCopyWith<$Res> {
  _$GridEditDto_AddSpanCopyWithImpl(this._self, this._then);

  final GridEditDto_AddSpan _self;
  final $Res Function(GridEditDto_AddSpan) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? lane = null,Object? first = null,Object? last = null,Object? technique = null,}) {
  return _then(GridEditDto_AddSpan(
lane: null == lane ? _self.lane : lane // ignore: cast_nullable_to_non_nullable
as PatternLaneDto,first: null == first ? _self.first : first // ignore: cast_nullable_to_non_nullable
as int,last: null == last ? _self.last : last // ignore: cast_nullable_to_non_nullable
as int,technique: null == technique ? _self.technique : technique // ignore: cast_nullable_to_non_nullable
as TechniqueKindDto,
  ));
}


}

/// @nodoc


class GridEditDto_RemoveSpan extends GridEditDto {
  const GridEditDto_RemoveSpan({required this.index}): super._();
  

 final  int index;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_RemoveSpanCopyWith<GridEditDto_RemoveSpan> get copyWith => _$GridEditDto_RemoveSpanCopyWithImpl<GridEditDto_RemoveSpan>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_RemoveSpan&&(identical(other.index, index) || other.index == index));
}


@override
int get hashCode {
    return Object.hash(runtimeType,index);
}

@override
String toString() {
    return 'GridEditDto.removeSpan(index: $index)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_RemoveSpanCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_RemoveSpanCopyWith(GridEditDto_RemoveSpan value, $Res Function(GridEditDto_RemoveSpan) _then) = _$GridEditDto_RemoveSpanCopyWithImpl;
@useResult
$Res call({
 int index
});




}
/// @nodoc
class _$GridEditDto_RemoveSpanCopyWithImpl<$Res>
    implements $GridEditDto_RemoveSpanCopyWith<$Res> {
  _$GridEditDto_RemoveSpanCopyWithImpl(this._self, this._then);

  final GridEditDto_RemoveSpan _self;
  final $Res Function(GridEditDto_RemoveSpan) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? index = null,}) {
  return _then(GridEditDto_RemoveSpan(
index: null == index ? _self.index : index // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class GridEditDto_SetLength extends GridEditDto {
  const GridEditDto_SetLength({required this.numerator, required this.denominator}): super._();
  

 final  PlatformInt64 numerator;
 final  PlatformInt64 denominator;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_SetLengthCopyWith<GridEditDto_SetLength> get copyWith => _$GridEditDto_SetLengthCopyWithImpl<GridEditDto_SetLength>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_SetLength&&(identical(other.numerator, numerator) || other.numerator == numerator)&&(identical(other.denominator, denominator) || other.denominator == denominator));
}


@override
int get hashCode {
    return Object.hash(runtimeType,numerator,denominator);
}

@override
String toString() {
    return 'GridEditDto.setLength(numerator: $numerator, denominator: $denominator)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_SetLengthCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_SetLengthCopyWith(GridEditDto_SetLength value, $Res Function(GridEditDto_SetLength) _then) = _$GridEditDto_SetLengthCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 numerator, PlatformInt64 denominator
});




}
/// @nodoc
class _$GridEditDto_SetLengthCopyWithImpl<$Res>
    implements $GridEditDto_SetLengthCopyWith<$Res> {
  _$GridEditDto_SetLengthCopyWithImpl(this._self, this._then);

  final GridEditDto_SetLength _self;
  final $Res Function(GridEditDto_SetLength) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? numerator = null,Object? denominator = null,}) {
  return _then(GridEditDto_SetLength(
numerator: null == numerator ? _self.numerator : numerator // ignore: cast_nullable_to_non_nullable
as PlatformInt64,denominator: null == denominator ? _self.denominator : denominator // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class GridEditDto_SetMeter extends GridEditDto {
  const GridEditDto_SetMeter({this.meter}): super._();
  

 final  MeterDto? meter;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_SetMeterCopyWith<GridEditDto_SetMeter> get copyWith => _$GridEditDto_SetMeterCopyWithImpl<GridEditDto_SetMeter>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_SetMeter&&(identical(other.meter, meter) || other.meter == meter));
}


@override
int get hashCode {
    return Object.hash(runtimeType,meter);
}

@override
String toString() {
    return 'GridEditDto.setMeter(meter: $meter)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_SetMeterCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_SetMeterCopyWith(GridEditDto_SetMeter value, $Res Function(GridEditDto_SetMeter) _then) = _$GridEditDto_SetMeterCopyWithImpl;
@useResult
$Res call({
 MeterDto? meter
});




}
/// @nodoc
class _$GridEditDto_SetMeterCopyWithImpl<$Res>
    implements $GridEditDto_SetMeterCopyWith<$Res> {
  _$GridEditDto_SetMeterCopyWithImpl(this._self, this._then);

  final GridEditDto_SetMeter _self;
  final $Res Function(GridEditDto_SetMeter) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? meter = freezed,}) {
  return _then(GridEditDto_SetMeter(
meter: freezed == meter ? _self.meter : meter // ignore: cast_nullable_to_non_nullable
as MeterDto?,
  ));
}


}

/// @nodoc


class GridEditDto_SetName extends GridEditDto {
  const GridEditDto_SetName({required this.name}): super._();
  

 final  String name;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_SetNameCopyWith<GridEditDto_SetName> get copyWith => _$GridEditDto_SetNameCopyWithImpl<GridEditDto_SetName>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_SetName&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode {
    return Object.hash(runtimeType,name);
}

@override
String toString() {
    return 'GridEditDto.setName(name: $name)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_SetNameCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_SetNameCopyWith(GridEditDto_SetName value, $Res Function(GridEditDto_SetName) _then) = _$GridEditDto_SetNameCopyWithImpl;
@useResult
$Res call({
 String name
});




}
/// @nodoc
class _$GridEditDto_SetNameCopyWithImpl<$Res>
    implements $GridEditDto_SetNameCopyWith<$Res> {
  _$GridEditDto_SetNameCopyWithImpl(this._self, this._then);

  final GridEditDto_SetName _self;
  final $Res Function(GridEditDto_SetName) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,}) {
  return _then(GridEditDto_SetName(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class GridEditDto_SetMeta extends GridEditDto {
  const GridEditDto_SetMeta({required this.description, required  List<String> tags}): _tags = tags,super._();
  

 final  String description;
 final  List<String> _tags;
 List<String> get tags {
  if (_tags is EqualUnmodifiableListView) return _tags;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_tags);
}


/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GridEditDto_SetMetaCopyWith<GridEditDto_SetMeta> get copyWith => _$GridEditDto_SetMetaCopyWithImpl<GridEditDto_SetMeta>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is GridEditDto_SetMeta&&(identical(other.description, description) || other.description == description)&&const DeepCollectionEquality().equals(other.tags, _tags));
}


@override
int get hashCode {
    return Object.hash(runtimeType,description,const DeepCollectionEquality().hash(_tags));
}

@override
String toString() {
    return 'GridEditDto.setMeta(description: $description, tags: $tags)';
}


}

/// @nodoc
abstract mixin class $GridEditDto_SetMetaCopyWith<$Res> implements $GridEditDtoCopyWith<$Res> {
  factory $GridEditDto_SetMetaCopyWith(GridEditDto_SetMeta value, $Res Function(GridEditDto_SetMeta) _then) = _$GridEditDto_SetMetaCopyWithImpl;
@useResult
$Res call({
 String description, List<String> tags
});




}
/// @nodoc
class _$GridEditDto_SetMetaCopyWithImpl<$Res>
    implements $GridEditDto_SetMetaCopyWith<$Res> {
  _$GridEditDto_SetMetaCopyWithImpl(this._self, this._then);

  final GridEditDto_SetMeta _self;
  final $Res Function(GridEditDto_SetMeta) _then;

/// Create a copy of GridEditDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? description = null,Object? tags = null,}) {
  return _then(GridEditDto_SetMeta(
description: null == description ? _self.description : description // ignore: cast_nullable_to_non_nullable
as String,tags: null == tags ? _self._tags : tags // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

// dart format on
