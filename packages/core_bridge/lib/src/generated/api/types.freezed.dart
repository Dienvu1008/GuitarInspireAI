// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'types.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$CommandDto {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CommandDto()';
}


}

/// @nodoc
class $CommandDtoCopyWith<$Res>  {
$CommandDtoCopyWith(CommandDto _, $Res Function(CommandDto) __);
}


/// Adds pattern-matching-related methods to [CommandDto].
extension CommandDtoPatterns on CommandDto {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( CommandDto_InsertMasterBars value)?  insertMasterBars,TResult Function( CommandDto_RemoveMasterBars value)?  removeMasterBars,TResult Function( CommandDto_SetMasterBar value)?  setMasterBar,TResult Function( CommandDto_SetTempo value)?  setTempo,TResult Function( CommandDto_RemoveTempo value)?  removeTempo,TResult Function( CommandDto_InsertBeat value)?  insertBeat,TResult Function( CommandDto_RemoveBeat value)?  removeBeat,TResult Function( CommandDto_SetBeatDuration value)?  setBeatDuration,TResult Function( CommandDto_SetNote value)?  setNote,TResult Function( CommandDto_RemoveNote value)?  removeNote,TResult Function( CommandDto_SetTie value)?  setTie,TResult Function( CommandDto_SetNoteEffects value)?  setNoteEffects,TResult Function( CommandDto_AddVoice value)?  addVoice,TResult Function( CommandDto_RemoveVoice value)?  removeVoice,TResult Function( CommandDto_ShiftFrets value)?  shiftFrets,TResult Function( CommandDto_AddTrack value)?  addTrack,TResult Function( CommandDto_RemoveTrack value)?  removeTrack,TResult Function( CommandDto_RenameTrack value)?  renameTrack,TResult Function( CommandDto_SetInstrument value)?  setInstrument,TResult Function( CommandDto_MoveTrack value)?  moveTrack,TResult Function( CommandDto_Retune value)?  retune,TResult Function( CommandDto_PasteBeats value)?  pasteBeats,TResult Function( CommandDto_RemoveBeats value)?  removeBeats,required TResult orElse(),}){
final _that = this;
switch (_that) {
case CommandDto_InsertMasterBars() when insertMasterBars != null:
return insertMasterBars(_that);case CommandDto_RemoveMasterBars() when removeMasterBars != null:
return removeMasterBars(_that);case CommandDto_SetMasterBar() when setMasterBar != null:
return setMasterBar(_that);case CommandDto_SetTempo() when setTempo != null:
return setTempo(_that);case CommandDto_RemoveTempo() when removeTempo != null:
return removeTempo(_that);case CommandDto_InsertBeat() when insertBeat != null:
return insertBeat(_that);case CommandDto_RemoveBeat() when removeBeat != null:
return removeBeat(_that);case CommandDto_SetBeatDuration() when setBeatDuration != null:
return setBeatDuration(_that);case CommandDto_SetNote() when setNote != null:
return setNote(_that);case CommandDto_RemoveNote() when removeNote != null:
return removeNote(_that);case CommandDto_SetTie() when setTie != null:
return setTie(_that);case CommandDto_SetNoteEffects() when setNoteEffects != null:
return setNoteEffects(_that);case CommandDto_AddVoice() when addVoice != null:
return addVoice(_that);case CommandDto_RemoveVoice() when removeVoice != null:
return removeVoice(_that);case CommandDto_ShiftFrets() when shiftFrets != null:
return shiftFrets(_that);case CommandDto_AddTrack() when addTrack != null:
return addTrack(_that);case CommandDto_RemoveTrack() when removeTrack != null:
return removeTrack(_that);case CommandDto_RenameTrack() when renameTrack != null:
return renameTrack(_that);case CommandDto_SetInstrument() when setInstrument != null:
return setInstrument(_that);case CommandDto_MoveTrack() when moveTrack != null:
return moveTrack(_that);case CommandDto_Retune() when retune != null:
return retune(_that);case CommandDto_PasteBeats() when pasteBeats != null:
return pasteBeats(_that);case CommandDto_RemoveBeats() when removeBeats != null:
return removeBeats(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( CommandDto_InsertMasterBars value)  insertMasterBars,required TResult Function( CommandDto_RemoveMasterBars value)  removeMasterBars,required TResult Function( CommandDto_SetMasterBar value)  setMasterBar,required TResult Function( CommandDto_SetTempo value)  setTempo,required TResult Function( CommandDto_RemoveTempo value)  removeTempo,required TResult Function( CommandDto_InsertBeat value)  insertBeat,required TResult Function( CommandDto_RemoveBeat value)  removeBeat,required TResult Function( CommandDto_SetBeatDuration value)  setBeatDuration,required TResult Function( CommandDto_SetNote value)  setNote,required TResult Function( CommandDto_RemoveNote value)  removeNote,required TResult Function( CommandDto_SetTie value)  setTie,required TResult Function( CommandDto_SetNoteEffects value)  setNoteEffects,required TResult Function( CommandDto_AddVoice value)  addVoice,required TResult Function( CommandDto_RemoveVoice value)  removeVoice,required TResult Function( CommandDto_ShiftFrets value)  shiftFrets,required TResult Function( CommandDto_AddTrack value)  addTrack,required TResult Function( CommandDto_RemoveTrack value)  removeTrack,required TResult Function( CommandDto_RenameTrack value)  renameTrack,required TResult Function( CommandDto_SetInstrument value)  setInstrument,required TResult Function( CommandDto_MoveTrack value)  moveTrack,required TResult Function( CommandDto_Retune value)  retune,required TResult Function( CommandDto_PasteBeats value)  pasteBeats,required TResult Function( CommandDto_RemoveBeats value)  removeBeats,}){
final _that = this;
switch (_that) {
case CommandDto_InsertMasterBars():
return insertMasterBars(_that);case CommandDto_RemoveMasterBars():
return removeMasterBars(_that);case CommandDto_SetMasterBar():
return setMasterBar(_that);case CommandDto_SetTempo():
return setTempo(_that);case CommandDto_RemoveTempo():
return removeTempo(_that);case CommandDto_InsertBeat():
return insertBeat(_that);case CommandDto_RemoveBeat():
return removeBeat(_that);case CommandDto_SetBeatDuration():
return setBeatDuration(_that);case CommandDto_SetNote():
return setNote(_that);case CommandDto_RemoveNote():
return removeNote(_that);case CommandDto_SetTie():
return setTie(_that);case CommandDto_SetNoteEffects():
return setNoteEffects(_that);case CommandDto_AddVoice():
return addVoice(_that);case CommandDto_RemoveVoice():
return removeVoice(_that);case CommandDto_ShiftFrets():
return shiftFrets(_that);case CommandDto_AddTrack():
return addTrack(_that);case CommandDto_RemoveTrack():
return removeTrack(_that);case CommandDto_RenameTrack():
return renameTrack(_that);case CommandDto_SetInstrument():
return setInstrument(_that);case CommandDto_MoveTrack():
return moveTrack(_that);case CommandDto_Retune():
return retune(_that);case CommandDto_PasteBeats():
return pasteBeats(_that);case CommandDto_RemoveBeats():
return removeBeats(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( CommandDto_InsertMasterBars value)?  insertMasterBars,TResult? Function( CommandDto_RemoveMasterBars value)?  removeMasterBars,TResult? Function( CommandDto_SetMasterBar value)?  setMasterBar,TResult? Function( CommandDto_SetTempo value)?  setTempo,TResult? Function( CommandDto_RemoveTempo value)?  removeTempo,TResult? Function( CommandDto_InsertBeat value)?  insertBeat,TResult? Function( CommandDto_RemoveBeat value)?  removeBeat,TResult? Function( CommandDto_SetBeatDuration value)?  setBeatDuration,TResult? Function( CommandDto_SetNote value)?  setNote,TResult? Function( CommandDto_RemoveNote value)?  removeNote,TResult? Function( CommandDto_SetTie value)?  setTie,TResult? Function( CommandDto_SetNoteEffects value)?  setNoteEffects,TResult? Function( CommandDto_AddVoice value)?  addVoice,TResult? Function( CommandDto_RemoveVoice value)?  removeVoice,TResult? Function( CommandDto_ShiftFrets value)?  shiftFrets,TResult? Function( CommandDto_AddTrack value)?  addTrack,TResult? Function( CommandDto_RemoveTrack value)?  removeTrack,TResult? Function( CommandDto_RenameTrack value)?  renameTrack,TResult? Function( CommandDto_SetInstrument value)?  setInstrument,TResult? Function( CommandDto_MoveTrack value)?  moveTrack,TResult? Function( CommandDto_Retune value)?  retune,TResult? Function( CommandDto_PasteBeats value)?  pasteBeats,TResult? Function( CommandDto_RemoveBeats value)?  removeBeats,}){
final _that = this;
switch (_that) {
case CommandDto_InsertMasterBars() when insertMasterBars != null:
return insertMasterBars(_that);case CommandDto_RemoveMasterBars() when removeMasterBars != null:
return removeMasterBars(_that);case CommandDto_SetMasterBar() when setMasterBar != null:
return setMasterBar(_that);case CommandDto_SetTempo() when setTempo != null:
return setTempo(_that);case CommandDto_RemoveTempo() when removeTempo != null:
return removeTempo(_that);case CommandDto_InsertBeat() when insertBeat != null:
return insertBeat(_that);case CommandDto_RemoveBeat() when removeBeat != null:
return removeBeat(_that);case CommandDto_SetBeatDuration() when setBeatDuration != null:
return setBeatDuration(_that);case CommandDto_SetNote() when setNote != null:
return setNote(_that);case CommandDto_RemoveNote() when removeNote != null:
return removeNote(_that);case CommandDto_SetTie() when setTie != null:
return setTie(_that);case CommandDto_SetNoteEffects() when setNoteEffects != null:
return setNoteEffects(_that);case CommandDto_AddVoice() when addVoice != null:
return addVoice(_that);case CommandDto_RemoveVoice() when removeVoice != null:
return removeVoice(_that);case CommandDto_ShiftFrets() when shiftFrets != null:
return shiftFrets(_that);case CommandDto_AddTrack() when addTrack != null:
return addTrack(_that);case CommandDto_RemoveTrack() when removeTrack != null:
return removeTrack(_that);case CommandDto_RenameTrack() when renameTrack != null:
return renameTrack(_that);case CommandDto_SetInstrument() when setInstrument != null:
return setInstrument(_that);case CommandDto_MoveTrack() when moveTrack != null:
return moveTrack(_that);case CommandDto_Retune() when retune != null:
return retune(_that);case CommandDto_PasteBeats() when pasteBeats != null:
return pasteBeats(_that);case CommandDto_RemoveBeats() when removeBeats != null:
return removeBeats(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( int at,  int count)?  insertMasterBars,TResult Function( int at,  int count)?  removeMasterBars,TResult Function( int index,  MasterBarDto bar)?  setMasterBar,TResult Function( int bar,  PlatformInt64 offsetNum,  PlatformInt64 offsetDen,  int bpm)?  setTempo,TResult Function( int bar,  PlatformInt64 offsetNum,  PlatformInt64 offsetDen)?  removeTempo,TResult Function( PlatformInt64 track,  int bar,  int voice,  int index,  DurationDto duration)?  insertBeat,TResult Function( BeatAddressDto beat)?  removeBeat,TResult Function( BeatAddressDto beat,  DurationDto duration)?  setBeatDuration,TResult Function( BeatAddressDto beat,  int string,  int fret)?  setNote,TResult Function( BeatAddressDto beat,  PlatformInt64 note)?  removeNote,TResult Function( BeatAddressDto beat,  PlatformInt64 note,  bool tie)?  setTie,TResult Function( BeatAddressDto beat,  PlatformInt64 note,  bool letRing,  bool dead)?  setNoteEffects,TResult Function( PlatformInt64 track,  int bar)?  addVoice,TResult Function( PlatformInt64 track,  int bar,  int voice)?  removeVoice,TResult Function( PlatformInt64 track,  int fromBar,  int toBar,  int delta)?  shiftFrets,TResult Function( String name,  InstrumentDto instrument)?  addTrack,TResult Function( PlatformInt64 track)?  removeTrack,TResult Function( PlatformInt64 track,  String name)?  renameTrack,TResult Function( PlatformInt64 track,  InstrumentDto instrument)?  setInstrument,TResult Function( PlatformInt64 track,  int to)?  moveTrack,TResult Function( PlatformInt64 track,  InstrumentDto instrument,  bool keepPitches)?  retune,TResult Function( PlatformInt64 track,  int bar,  int voice,  int index,  List<ClipBeatDto> beats)?  pasteBeats,TResult Function( PlatformInt64 track,  int voice,  int fromBar,  int fromIndex,  int toBar,  int toIndex)?  removeBeats,required TResult orElse(),}) {final _that = this;
switch (_that) {
case CommandDto_InsertMasterBars() when insertMasterBars != null:
return insertMasterBars(_that.at,_that.count);case CommandDto_RemoveMasterBars() when removeMasterBars != null:
return removeMasterBars(_that.at,_that.count);case CommandDto_SetMasterBar() when setMasterBar != null:
return setMasterBar(_that.index,_that.bar);case CommandDto_SetTempo() when setTempo != null:
return setTempo(_that.bar,_that.offsetNum,_that.offsetDen,_that.bpm);case CommandDto_RemoveTempo() when removeTempo != null:
return removeTempo(_that.bar,_that.offsetNum,_that.offsetDen);case CommandDto_InsertBeat() when insertBeat != null:
return insertBeat(_that.track,_that.bar,_that.voice,_that.index,_that.duration);case CommandDto_RemoveBeat() when removeBeat != null:
return removeBeat(_that.beat);case CommandDto_SetBeatDuration() when setBeatDuration != null:
return setBeatDuration(_that.beat,_that.duration);case CommandDto_SetNote() when setNote != null:
return setNote(_that.beat,_that.string,_that.fret);case CommandDto_RemoveNote() when removeNote != null:
return removeNote(_that.beat,_that.note);case CommandDto_SetTie() when setTie != null:
return setTie(_that.beat,_that.note,_that.tie);case CommandDto_SetNoteEffects() when setNoteEffects != null:
return setNoteEffects(_that.beat,_that.note,_that.letRing,_that.dead);case CommandDto_AddVoice() when addVoice != null:
return addVoice(_that.track,_that.bar);case CommandDto_RemoveVoice() when removeVoice != null:
return removeVoice(_that.track,_that.bar,_that.voice);case CommandDto_ShiftFrets() when shiftFrets != null:
return shiftFrets(_that.track,_that.fromBar,_that.toBar,_that.delta);case CommandDto_AddTrack() when addTrack != null:
return addTrack(_that.name,_that.instrument);case CommandDto_RemoveTrack() when removeTrack != null:
return removeTrack(_that.track);case CommandDto_RenameTrack() when renameTrack != null:
return renameTrack(_that.track,_that.name);case CommandDto_SetInstrument() when setInstrument != null:
return setInstrument(_that.track,_that.instrument);case CommandDto_MoveTrack() when moveTrack != null:
return moveTrack(_that.track,_that.to);case CommandDto_Retune() when retune != null:
return retune(_that.track,_that.instrument,_that.keepPitches);case CommandDto_PasteBeats() when pasteBeats != null:
return pasteBeats(_that.track,_that.bar,_that.voice,_that.index,_that.beats);case CommandDto_RemoveBeats() when removeBeats != null:
return removeBeats(_that.track,_that.voice,_that.fromBar,_that.fromIndex,_that.toBar,_that.toIndex);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( int at,  int count)  insertMasterBars,required TResult Function( int at,  int count)  removeMasterBars,required TResult Function( int index,  MasterBarDto bar)  setMasterBar,required TResult Function( int bar,  PlatformInt64 offsetNum,  PlatformInt64 offsetDen,  int bpm)  setTempo,required TResult Function( int bar,  PlatformInt64 offsetNum,  PlatformInt64 offsetDen)  removeTempo,required TResult Function( PlatformInt64 track,  int bar,  int voice,  int index,  DurationDto duration)  insertBeat,required TResult Function( BeatAddressDto beat)  removeBeat,required TResult Function( BeatAddressDto beat,  DurationDto duration)  setBeatDuration,required TResult Function( BeatAddressDto beat,  int string,  int fret)  setNote,required TResult Function( BeatAddressDto beat,  PlatformInt64 note)  removeNote,required TResult Function( BeatAddressDto beat,  PlatformInt64 note,  bool tie)  setTie,required TResult Function( BeatAddressDto beat,  PlatformInt64 note,  bool letRing,  bool dead)  setNoteEffects,required TResult Function( PlatformInt64 track,  int bar)  addVoice,required TResult Function( PlatformInt64 track,  int bar,  int voice)  removeVoice,required TResult Function( PlatformInt64 track,  int fromBar,  int toBar,  int delta)  shiftFrets,required TResult Function( String name,  InstrumentDto instrument)  addTrack,required TResult Function( PlatformInt64 track)  removeTrack,required TResult Function( PlatformInt64 track,  String name)  renameTrack,required TResult Function( PlatformInt64 track,  InstrumentDto instrument)  setInstrument,required TResult Function( PlatformInt64 track,  int to)  moveTrack,required TResult Function( PlatformInt64 track,  InstrumentDto instrument,  bool keepPitches)  retune,required TResult Function( PlatformInt64 track,  int bar,  int voice,  int index,  List<ClipBeatDto> beats)  pasteBeats,required TResult Function( PlatformInt64 track,  int voice,  int fromBar,  int fromIndex,  int toBar,  int toIndex)  removeBeats,}) {final _that = this;
switch (_that) {
case CommandDto_InsertMasterBars():
return insertMasterBars(_that.at,_that.count);case CommandDto_RemoveMasterBars():
return removeMasterBars(_that.at,_that.count);case CommandDto_SetMasterBar():
return setMasterBar(_that.index,_that.bar);case CommandDto_SetTempo():
return setTempo(_that.bar,_that.offsetNum,_that.offsetDen,_that.bpm);case CommandDto_RemoveTempo():
return removeTempo(_that.bar,_that.offsetNum,_that.offsetDen);case CommandDto_InsertBeat():
return insertBeat(_that.track,_that.bar,_that.voice,_that.index,_that.duration);case CommandDto_RemoveBeat():
return removeBeat(_that.beat);case CommandDto_SetBeatDuration():
return setBeatDuration(_that.beat,_that.duration);case CommandDto_SetNote():
return setNote(_that.beat,_that.string,_that.fret);case CommandDto_RemoveNote():
return removeNote(_that.beat,_that.note);case CommandDto_SetTie():
return setTie(_that.beat,_that.note,_that.tie);case CommandDto_SetNoteEffects():
return setNoteEffects(_that.beat,_that.note,_that.letRing,_that.dead);case CommandDto_AddVoice():
return addVoice(_that.track,_that.bar);case CommandDto_RemoveVoice():
return removeVoice(_that.track,_that.bar,_that.voice);case CommandDto_ShiftFrets():
return shiftFrets(_that.track,_that.fromBar,_that.toBar,_that.delta);case CommandDto_AddTrack():
return addTrack(_that.name,_that.instrument);case CommandDto_RemoveTrack():
return removeTrack(_that.track);case CommandDto_RenameTrack():
return renameTrack(_that.track,_that.name);case CommandDto_SetInstrument():
return setInstrument(_that.track,_that.instrument);case CommandDto_MoveTrack():
return moveTrack(_that.track,_that.to);case CommandDto_Retune():
return retune(_that.track,_that.instrument,_that.keepPitches);case CommandDto_PasteBeats():
return pasteBeats(_that.track,_that.bar,_that.voice,_that.index,_that.beats);case CommandDto_RemoveBeats():
return removeBeats(_that.track,_that.voice,_that.fromBar,_that.fromIndex,_that.toBar,_that.toIndex);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( int at,  int count)?  insertMasterBars,TResult? Function( int at,  int count)?  removeMasterBars,TResult? Function( int index,  MasterBarDto bar)?  setMasterBar,TResult? Function( int bar,  PlatformInt64 offsetNum,  PlatformInt64 offsetDen,  int bpm)?  setTempo,TResult? Function( int bar,  PlatformInt64 offsetNum,  PlatformInt64 offsetDen)?  removeTempo,TResult? Function( PlatformInt64 track,  int bar,  int voice,  int index,  DurationDto duration)?  insertBeat,TResult? Function( BeatAddressDto beat)?  removeBeat,TResult? Function( BeatAddressDto beat,  DurationDto duration)?  setBeatDuration,TResult? Function( BeatAddressDto beat,  int string,  int fret)?  setNote,TResult? Function( BeatAddressDto beat,  PlatformInt64 note)?  removeNote,TResult? Function( BeatAddressDto beat,  PlatformInt64 note,  bool tie)?  setTie,TResult? Function( BeatAddressDto beat,  PlatformInt64 note,  bool letRing,  bool dead)?  setNoteEffects,TResult? Function( PlatformInt64 track,  int bar)?  addVoice,TResult? Function( PlatformInt64 track,  int bar,  int voice)?  removeVoice,TResult? Function( PlatformInt64 track,  int fromBar,  int toBar,  int delta)?  shiftFrets,TResult? Function( String name,  InstrumentDto instrument)?  addTrack,TResult? Function( PlatformInt64 track)?  removeTrack,TResult? Function( PlatformInt64 track,  String name)?  renameTrack,TResult? Function( PlatformInt64 track,  InstrumentDto instrument)?  setInstrument,TResult? Function( PlatformInt64 track,  int to)?  moveTrack,TResult? Function( PlatformInt64 track,  InstrumentDto instrument,  bool keepPitches)?  retune,TResult? Function( PlatformInt64 track,  int bar,  int voice,  int index,  List<ClipBeatDto> beats)?  pasteBeats,TResult? Function( PlatformInt64 track,  int voice,  int fromBar,  int fromIndex,  int toBar,  int toIndex)?  removeBeats,}) {final _that = this;
switch (_that) {
case CommandDto_InsertMasterBars() when insertMasterBars != null:
return insertMasterBars(_that.at,_that.count);case CommandDto_RemoveMasterBars() when removeMasterBars != null:
return removeMasterBars(_that.at,_that.count);case CommandDto_SetMasterBar() when setMasterBar != null:
return setMasterBar(_that.index,_that.bar);case CommandDto_SetTempo() when setTempo != null:
return setTempo(_that.bar,_that.offsetNum,_that.offsetDen,_that.bpm);case CommandDto_RemoveTempo() when removeTempo != null:
return removeTempo(_that.bar,_that.offsetNum,_that.offsetDen);case CommandDto_InsertBeat() when insertBeat != null:
return insertBeat(_that.track,_that.bar,_that.voice,_that.index,_that.duration);case CommandDto_RemoveBeat() when removeBeat != null:
return removeBeat(_that.beat);case CommandDto_SetBeatDuration() when setBeatDuration != null:
return setBeatDuration(_that.beat,_that.duration);case CommandDto_SetNote() when setNote != null:
return setNote(_that.beat,_that.string,_that.fret);case CommandDto_RemoveNote() when removeNote != null:
return removeNote(_that.beat,_that.note);case CommandDto_SetTie() when setTie != null:
return setTie(_that.beat,_that.note,_that.tie);case CommandDto_SetNoteEffects() when setNoteEffects != null:
return setNoteEffects(_that.beat,_that.note,_that.letRing,_that.dead);case CommandDto_AddVoice() when addVoice != null:
return addVoice(_that.track,_that.bar);case CommandDto_RemoveVoice() when removeVoice != null:
return removeVoice(_that.track,_that.bar,_that.voice);case CommandDto_ShiftFrets() when shiftFrets != null:
return shiftFrets(_that.track,_that.fromBar,_that.toBar,_that.delta);case CommandDto_AddTrack() when addTrack != null:
return addTrack(_that.name,_that.instrument);case CommandDto_RemoveTrack() when removeTrack != null:
return removeTrack(_that.track);case CommandDto_RenameTrack() when renameTrack != null:
return renameTrack(_that.track,_that.name);case CommandDto_SetInstrument() when setInstrument != null:
return setInstrument(_that.track,_that.instrument);case CommandDto_MoveTrack() when moveTrack != null:
return moveTrack(_that.track,_that.to);case CommandDto_Retune() when retune != null:
return retune(_that.track,_that.instrument,_that.keepPitches);case CommandDto_PasteBeats() when pasteBeats != null:
return pasteBeats(_that.track,_that.bar,_that.voice,_that.index,_that.beats);case CommandDto_RemoveBeats() when removeBeats != null:
return removeBeats(_that.track,_that.voice,_that.fromBar,_that.fromIndex,_that.toBar,_that.toIndex);case _:
  return null;

}
}

}

/// @nodoc


class CommandDto_InsertMasterBars extends CommandDto {
  const CommandDto_InsertMasterBars({required this.at, required this.count}): super._();
  

 final  int at;
 final  int count;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_InsertMasterBarsCopyWith<CommandDto_InsertMasterBars> get copyWith => _$CommandDto_InsertMasterBarsCopyWithImpl<CommandDto_InsertMasterBars>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_InsertMasterBars&&(identical(other.at, at) || other.at == at)&&(identical(other.count, count) || other.count == count));
}


@override
int get hashCode {
    return Object.hash(runtimeType,at,count);
}

@override
String toString() {
    return 'CommandDto.insertMasterBars(at: $at, count: $count)';
}


}

/// @nodoc
abstract mixin class $CommandDto_InsertMasterBarsCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_InsertMasterBarsCopyWith(CommandDto_InsertMasterBars value, $Res Function(CommandDto_InsertMasterBars) _then) = _$CommandDto_InsertMasterBarsCopyWithImpl;
@useResult
$Res call({
 int at, int count
});




}
/// @nodoc
class _$CommandDto_InsertMasterBarsCopyWithImpl<$Res>
    implements $CommandDto_InsertMasterBarsCopyWith<$Res> {
  _$CommandDto_InsertMasterBarsCopyWithImpl(this._self, this._then);

  final CommandDto_InsertMasterBars _self;
  final $Res Function(CommandDto_InsertMasterBars) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? at = null,Object? count = null,}) {
  return _then(CommandDto_InsertMasterBars(
at: null == at ? _self.at : at // ignore: cast_nullable_to_non_nullable
as int,count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class CommandDto_RemoveMasterBars extends CommandDto {
  const CommandDto_RemoveMasterBars({required this.at, required this.count}): super._();
  

 final  int at;
 final  int count;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_RemoveMasterBarsCopyWith<CommandDto_RemoveMasterBars> get copyWith => _$CommandDto_RemoveMasterBarsCopyWithImpl<CommandDto_RemoveMasterBars>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_RemoveMasterBars&&(identical(other.at, at) || other.at == at)&&(identical(other.count, count) || other.count == count));
}


@override
int get hashCode {
    return Object.hash(runtimeType,at,count);
}

@override
String toString() {
    return 'CommandDto.removeMasterBars(at: $at, count: $count)';
}


}

/// @nodoc
abstract mixin class $CommandDto_RemoveMasterBarsCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_RemoveMasterBarsCopyWith(CommandDto_RemoveMasterBars value, $Res Function(CommandDto_RemoveMasterBars) _then) = _$CommandDto_RemoveMasterBarsCopyWithImpl;
@useResult
$Res call({
 int at, int count
});




}
/// @nodoc
class _$CommandDto_RemoveMasterBarsCopyWithImpl<$Res>
    implements $CommandDto_RemoveMasterBarsCopyWith<$Res> {
  _$CommandDto_RemoveMasterBarsCopyWithImpl(this._self, this._then);

  final CommandDto_RemoveMasterBars _self;
  final $Res Function(CommandDto_RemoveMasterBars) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? at = null,Object? count = null,}) {
  return _then(CommandDto_RemoveMasterBars(
at: null == at ? _self.at : at // ignore: cast_nullable_to_non_nullable
as int,count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class CommandDto_SetMasterBar extends CommandDto {
  const CommandDto_SetMasterBar({required this.index, required this.bar}): super._();
  

 final  int index;
 final  MasterBarDto bar;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_SetMasterBarCopyWith<CommandDto_SetMasterBar> get copyWith => _$CommandDto_SetMasterBarCopyWithImpl<CommandDto_SetMasterBar>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_SetMasterBar&&(identical(other.index, index) || other.index == index)&&(identical(other.bar, bar) || other.bar == bar));
}


@override
int get hashCode {
    return Object.hash(runtimeType,index,bar);
}

@override
String toString() {
    return 'CommandDto.setMasterBar(index: $index, bar: $bar)';
}


}

/// @nodoc
abstract mixin class $CommandDto_SetMasterBarCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_SetMasterBarCopyWith(CommandDto_SetMasterBar value, $Res Function(CommandDto_SetMasterBar) _then) = _$CommandDto_SetMasterBarCopyWithImpl;
@useResult
$Res call({
 int index, MasterBarDto bar
});




}
/// @nodoc
class _$CommandDto_SetMasterBarCopyWithImpl<$Res>
    implements $CommandDto_SetMasterBarCopyWith<$Res> {
  _$CommandDto_SetMasterBarCopyWithImpl(this._self, this._then);

  final CommandDto_SetMasterBar _self;
  final $Res Function(CommandDto_SetMasterBar) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? index = null,Object? bar = null,}) {
  return _then(CommandDto_SetMasterBar(
index: null == index ? _self.index : index // ignore: cast_nullable_to_non_nullable
as int,bar: null == bar ? _self.bar : bar // ignore: cast_nullable_to_non_nullable
as MasterBarDto,
  ));
}


}

/// @nodoc


class CommandDto_SetTempo extends CommandDto {
  const CommandDto_SetTempo({required this.bar, required this.offsetNum, required this.offsetDen, required this.bpm}): super._();
  

 final  int bar;
 final  PlatformInt64 offsetNum;
 final  PlatformInt64 offsetDen;
 final  int bpm;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_SetTempoCopyWith<CommandDto_SetTempo> get copyWith => _$CommandDto_SetTempoCopyWithImpl<CommandDto_SetTempo>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_SetTempo&&(identical(other.bar, bar) || other.bar == bar)&&(identical(other.offsetNum, offsetNum) || other.offsetNum == offsetNum)&&(identical(other.offsetDen, offsetDen) || other.offsetDen == offsetDen)&&(identical(other.bpm, bpm) || other.bpm == bpm));
}


@override
int get hashCode {
    return Object.hash(runtimeType,bar,offsetNum,offsetDen,bpm);
}

@override
String toString() {
    return 'CommandDto.setTempo(bar: $bar, offsetNum: $offsetNum, offsetDen: $offsetDen, bpm: $bpm)';
}


}

/// @nodoc
abstract mixin class $CommandDto_SetTempoCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_SetTempoCopyWith(CommandDto_SetTempo value, $Res Function(CommandDto_SetTempo) _then) = _$CommandDto_SetTempoCopyWithImpl;
@useResult
$Res call({
 int bar, PlatformInt64 offsetNum, PlatformInt64 offsetDen, int bpm
});




}
/// @nodoc
class _$CommandDto_SetTempoCopyWithImpl<$Res>
    implements $CommandDto_SetTempoCopyWith<$Res> {
  _$CommandDto_SetTempoCopyWithImpl(this._self, this._then);

  final CommandDto_SetTempo _self;
  final $Res Function(CommandDto_SetTempo) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? bar = null,Object? offsetNum = null,Object? offsetDen = null,Object? bpm = null,}) {
  return _then(CommandDto_SetTempo(
bar: null == bar ? _self.bar : bar // ignore: cast_nullable_to_non_nullable
as int,offsetNum: null == offsetNum ? _self.offsetNum : offsetNum // ignore: cast_nullable_to_non_nullable
as PlatformInt64,offsetDen: null == offsetDen ? _self.offsetDen : offsetDen // ignore: cast_nullable_to_non_nullable
as PlatformInt64,bpm: null == bpm ? _self.bpm : bpm // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class CommandDto_RemoveTempo extends CommandDto {
  const CommandDto_RemoveTempo({required this.bar, required this.offsetNum, required this.offsetDen}): super._();
  

 final  int bar;
 final  PlatformInt64 offsetNum;
 final  PlatformInt64 offsetDen;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_RemoveTempoCopyWith<CommandDto_RemoveTempo> get copyWith => _$CommandDto_RemoveTempoCopyWithImpl<CommandDto_RemoveTempo>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_RemoveTempo&&(identical(other.bar, bar) || other.bar == bar)&&(identical(other.offsetNum, offsetNum) || other.offsetNum == offsetNum)&&(identical(other.offsetDen, offsetDen) || other.offsetDen == offsetDen));
}


@override
int get hashCode {
    return Object.hash(runtimeType,bar,offsetNum,offsetDen);
}

@override
String toString() {
    return 'CommandDto.removeTempo(bar: $bar, offsetNum: $offsetNum, offsetDen: $offsetDen)';
}


}

/// @nodoc
abstract mixin class $CommandDto_RemoveTempoCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_RemoveTempoCopyWith(CommandDto_RemoveTempo value, $Res Function(CommandDto_RemoveTempo) _then) = _$CommandDto_RemoveTempoCopyWithImpl;
@useResult
$Res call({
 int bar, PlatformInt64 offsetNum, PlatformInt64 offsetDen
});




}
/// @nodoc
class _$CommandDto_RemoveTempoCopyWithImpl<$Res>
    implements $CommandDto_RemoveTempoCopyWith<$Res> {
  _$CommandDto_RemoveTempoCopyWithImpl(this._self, this._then);

  final CommandDto_RemoveTempo _self;
  final $Res Function(CommandDto_RemoveTempo) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? bar = null,Object? offsetNum = null,Object? offsetDen = null,}) {
  return _then(CommandDto_RemoveTempo(
bar: null == bar ? _self.bar : bar // ignore: cast_nullable_to_non_nullable
as int,offsetNum: null == offsetNum ? _self.offsetNum : offsetNum // ignore: cast_nullable_to_non_nullable
as PlatformInt64,offsetDen: null == offsetDen ? _self.offsetDen : offsetDen // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class CommandDto_InsertBeat extends CommandDto {
  const CommandDto_InsertBeat({required this.track, required this.bar, required this.voice, required this.index, required this.duration}): super._();
  

 final  PlatformInt64 track;
 final  int bar;
 final  int voice;
 final  int index;
 final  DurationDto duration;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_InsertBeatCopyWith<CommandDto_InsertBeat> get copyWith => _$CommandDto_InsertBeatCopyWithImpl<CommandDto_InsertBeat>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_InsertBeat&&(identical(other.track, track) || other.track == track)&&(identical(other.bar, bar) || other.bar == bar)&&(identical(other.voice, voice) || other.voice == voice)&&(identical(other.index, index) || other.index == index)&&(identical(other.duration, duration) || other.duration == duration));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,bar,voice,index,duration);
}

@override
String toString() {
    return 'CommandDto.insertBeat(track: $track, bar: $bar, voice: $voice, index: $index, duration: $duration)';
}


}

/// @nodoc
abstract mixin class $CommandDto_InsertBeatCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_InsertBeatCopyWith(CommandDto_InsertBeat value, $Res Function(CommandDto_InsertBeat) _then) = _$CommandDto_InsertBeatCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, int bar, int voice, int index, DurationDto duration
});




}
/// @nodoc
class _$CommandDto_InsertBeatCopyWithImpl<$Res>
    implements $CommandDto_InsertBeatCopyWith<$Res> {
  _$CommandDto_InsertBeatCopyWithImpl(this._self, this._then);

  final CommandDto_InsertBeat _self;
  final $Res Function(CommandDto_InsertBeat) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? bar = null,Object? voice = null,Object? index = null,Object? duration = null,}) {
  return _then(CommandDto_InsertBeat(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,bar: null == bar ? _self.bar : bar // ignore: cast_nullable_to_non_nullable
as int,voice: null == voice ? _self.voice : voice // ignore: cast_nullable_to_non_nullable
as int,index: null == index ? _self.index : index // ignore: cast_nullable_to_non_nullable
as int,duration: null == duration ? _self.duration : duration // ignore: cast_nullable_to_non_nullable
as DurationDto,
  ));
}


}

/// @nodoc


class CommandDto_RemoveBeat extends CommandDto {
  const CommandDto_RemoveBeat({required this.beat}): super._();
  

 final  BeatAddressDto beat;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_RemoveBeatCopyWith<CommandDto_RemoveBeat> get copyWith => _$CommandDto_RemoveBeatCopyWithImpl<CommandDto_RemoveBeat>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_RemoveBeat&&(identical(other.beat, beat) || other.beat == beat));
}


@override
int get hashCode {
    return Object.hash(runtimeType,beat);
}

@override
String toString() {
    return 'CommandDto.removeBeat(beat: $beat)';
}


}

/// @nodoc
abstract mixin class $CommandDto_RemoveBeatCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_RemoveBeatCopyWith(CommandDto_RemoveBeat value, $Res Function(CommandDto_RemoveBeat) _then) = _$CommandDto_RemoveBeatCopyWithImpl;
@useResult
$Res call({
 BeatAddressDto beat
});




}
/// @nodoc
class _$CommandDto_RemoveBeatCopyWithImpl<$Res>
    implements $CommandDto_RemoveBeatCopyWith<$Res> {
  _$CommandDto_RemoveBeatCopyWithImpl(this._self, this._then);

  final CommandDto_RemoveBeat _self;
  final $Res Function(CommandDto_RemoveBeat) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? beat = null,}) {
  return _then(CommandDto_RemoveBeat(
beat: null == beat ? _self.beat : beat // ignore: cast_nullable_to_non_nullable
as BeatAddressDto,
  ));
}


}

/// @nodoc


class CommandDto_SetBeatDuration extends CommandDto {
  const CommandDto_SetBeatDuration({required this.beat, required this.duration}): super._();
  

 final  BeatAddressDto beat;
 final  DurationDto duration;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_SetBeatDurationCopyWith<CommandDto_SetBeatDuration> get copyWith => _$CommandDto_SetBeatDurationCopyWithImpl<CommandDto_SetBeatDuration>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_SetBeatDuration&&(identical(other.beat, beat) || other.beat == beat)&&(identical(other.duration, duration) || other.duration == duration));
}


@override
int get hashCode {
    return Object.hash(runtimeType,beat,duration);
}

@override
String toString() {
    return 'CommandDto.setBeatDuration(beat: $beat, duration: $duration)';
}


}

/// @nodoc
abstract mixin class $CommandDto_SetBeatDurationCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_SetBeatDurationCopyWith(CommandDto_SetBeatDuration value, $Res Function(CommandDto_SetBeatDuration) _then) = _$CommandDto_SetBeatDurationCopyWithImpl;
@useResult
$Res call({
 BeatAddressDto beat, DurationDto duration
});




}
/// @nodoc
class _$CommandDto_SetBeatDurationCopyWithImpl<$Res>
    implements $CommandDto_SetBeatDurationCopyWith<$Res> {
  _$CommandDto_SetBeatDurationCopyWithImpl(this._self, this._then);

  final CommandDto_SetBeatDuration _self;
  final $Res Function(CommandDto_SetBeatDuration) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? beat = null,Object? duration = null,}) {
  return _then(CommandDto_SetBeatDuration(
beat: null == beat ? _self.beat : beat // ignore: cast_nullable_to_non_nullable
as BeatAddressDto,duration: null == duration ? _self.duration : duration // ignore: cast_nullable_to_non_nullable
as DurationDto,
  ));
}


}

/// @nodoc


class CommandDto_SetNote extends CommandDto {
  const CommandDto_SetNote({required this.beat, required this.string, required this.fret}): super._();
  

 final  BeatAddressDto beat;
 final  int string;
 final  int fret;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_SetNoteCopyWith<CommandDto_SetNote> get copyWith => _$CommandDto_SetNoteCopyWithImpl<CommandDto_SetNote>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_SetNote&&(identical(other.beat, beat) || other.beat == beat)&&(identical(other.string, string) || other.string == string)&&(identical(other.fret, fret) || other.fret == fret));
}


@override
int get hashCode {
    return Object.hash(runtimeType,beat,string,fret);
}

@override
String toString() {
    return 'CommandDto.setNote(beat: $beat, string: $string, fret: $fret)';
}


}

/// @nodoc
abstract mixin class $CommandDto_SetNoteCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_SetNoteCopyWith(CommandDto_SetNote value, $Res Function(CommandDto_SetNote) _then) = _$CommandDto_SetNoteCopyWithImpl;
@useResult
$Res call({
 BeatAddressDto beat, int string, int fret
});




}
/// @nodoc
class _$CommandDto_SetNoteCopyWithImpl<$Res>
    implements $CommandDto_SetNoteCopyWith<$Res> {
  _$CommandDto_SetNoteCopyWithImpl(this._self, this._then);

  final CommandDto_SetNote _self;
  final $Res Function(CommandDto_SetNote) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? beat = null,Object? string = null,Object? fret = null,}) {
  return _then(CommandDto_SetNote(
beat: null == beat ? _self.beat : beat // ignore: cast_nullable_to_non_nullable
as BeatAddressDto,string: null == string ? _self.string : string // ignore: cast_nullable_to_non_nullable
as int,fret: null == fret ? _self.fret : fret // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class CommandDto_RemoveNote extends CommandDto {
  const CommandDto_RemoveNote({required this.beat, required this.note}): super._();
  

 final  BeatAddressDto beat;
 final  PlatformInt64 note;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_RemoveNoteCopyWith<CommandDto_RemoveNote> get copyWith => _$CommandDto_RemoveNoteCopyWithImpl<CommandDto_RemoveNote>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_RemoveNote&&(identical(other.beat, beat) || other.beat == beat)&&(identical(other.note, note) || other.note == note));
}


@override
int get hashCode {
    return Object.hash(runtimeType,beat,note);
}

@override
String toString() {
    return 'CommandDto.removeNote(beat: $beat, note: $note)';
}


}

/// @nodoc
abstract mixin class $CommandDto_RemoveNoteCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_RemoveNoteCopyWith(CommandDto_RemoveNote value, $Res Function(CommandDto_RemoveNote) _then) = _$CommandDto_RemoveNoteCopyWithImpl;
@useResult
$Res call({
 BeatAddressDto beat, PlatformInt64 note
});




}
/// @nodoc
class _$CommandDto_RemoveNoteCopyWithImpl<$Res>
    implements $CommandDto_RemoveNoteCopyWith<$Res> {
  _$CommandDto_RemoveNoteCopyWithImpl(this._self, this._then);

  final CommandDto_RemoveNote _self;
  final $Res Function(CommandDto_RemoveNote) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? beat = null,Object? note = null,}) {
  return _then(CommandDto_RemoveNote(
beat: null == beat ? _self.beat : beat // ignore: cast_nullable_to_non_nullable
as BeatAddressDto,note: null == note ? _self.note : note // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class CommandDto_SetTie extends CommandDto {
  const CommandDto_SetTie({required this.beat, required this.note, required this.tie}): super._();
  

 final  BeatAddressDto beat;
 final  PlatformInt64 note;
 final  bool tie;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_SetTieCopyWith<CommandDto_SetTie> get copyWith => _$CommandDto_SetTieCopyWithImpl<CommandDto_SetTie>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_SetTie&&(identical(other.beat, beat) || other.beat == beat)&&(identical(other.note, note) || other.note == note)&&(identical(other.tie, tie) || other.tie == tie));
}


@override
int get hashCode {
    return Object.hash(runtimeType,beat,note,tie);
}

@override
String toString() {
    return 'CommandDto.setTie(beat: $beat, note: $note, tie: $tie)';
}


}

/// @nodoc
abstract mixin class $CommandDto_SetTieCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_SetTieCopyWith(CommandDto_SetTie value, $Res Function(CommandDto_SetTie) _then) = _$CommandDto_SetTieCopyWithImpl;
@useResult
$Res call({
 BeatAddressDto beat, PlatformInt64 note, bool tie
});




}
/// @nodoc
class _$CommandDto_SetTieCopyWithImpl<$Res>
    implements $CommandDto_SetTieCopyWith<$Res> {
  _$CommandDto_SetTieCopyWithImpl(this._self, this._then);

  final CommandDto_SetTie _self;
  final $Res Function(CommandDto_SetTie) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? beat = null,Object? note = null,Object? tie = null,}) {
  return _then(CommandDto_SetTie(
beat: null == beat ? _self.beat : beat // ignore: cast_nullable_to_non_nullable
as BeatAddressDto,note: null == note ? _self.note : note // ignore: cast_nullable_to_non_nullable
as PlatformInt64,tie: null == tie ? _self.tie : tie // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class CommandDto_SetNoteEffects extends CommandDto {
  const CommandDto_SetNoteEffects({required this.beat, required this.note, required this.letRing, required this.dead}): super._();
  

 final  BeatAddressDto beat;
 final  PlatformInt64 note;
 final  bool letRing;
 final  bool dead;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_SetNoteEffectsCopyWith<CommandDto_SetNoteEffects> get copyWith => _$CommandDto_SetNoteEffectsCopyWithImpl<CommandDto_SetNoteEffects>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_SetNoteEffects&&(identical(other.beat, beat) || other.beat == beat)&&(identical(other.note, note) || other.note == note)&&(identical(other.letRing, letRing) || other.letRing == letRing)&&(identical(other.dead, dead) || other.dead == dead));
}


@override
int get hashCode {
    return Object.hash(runtimeType,beat,note,letRing,dead);
}

@override
String toString() {
    return 'CommandDto.setNoteEffects(beat: $beat, note: $note, letRing: $letRing, dead: $dead)';
}


}

/// @nodoc
abstract mixin class $CommandDto_SetNoteEffectsCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_SetNoteEffectsCopyWith(CommandDto_SetNoteEffects value, $Res Function(CommandDto_SetNoteEffects) _then) = _$CommandDto_SetNoteEffectsCopyWithImpl;
@useResult
$Res call({
 BeatAddressDto beat, PlatformInt64 note, bool letRing, bool dead
});




}
/// @nodoc
class _$CommandDto_SetNoteEffectsCopyWithImpl<$Res>
    implements $CommandDto_SetNoteEffectsCopyWith<$Res> {
  _$CommandDto_SetNoteEffectsCopyWithImpl(this._self, this._then);

  final CommandDto_SetNoteEffects _self;
  final $Res Function(CommandDto_SetNoteEffects) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? beat = null,Object? note = null,Object? letRing = null,Object? dead = null,}) {
  return _then(CommandDto_SetNoteEffects(
beat: null == beat ? _self.beat : beat // ignore: cast_nullable_to_non_nullable
as BeatAddressDto,note: null == note ? _self.note : note // ignore: cast_nullable_to_non_nullable
as PlatformInt64,letRing: null == letRing ? _self.letRing : letRing // ignore: cast_nullable_to_non_nullable
as bool,dead: null == dead ? _self.dead : dead // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class CommandDto_AddVoice extends CommandDto {
  const CommandDto_AddVoice({required this.track, required this.bar}): super._();
  

 final  PlatformInt64 track;
 final  int bar;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_AddVoiceCopyWith<CommandDto_AddVoice> get copyWith => _$CommandDto_AddVoiceCopyWithImpl<CommandDto_AddVoice>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_AddVoice&&(identical(other.track, track) || other.track == track)&&(identical(other.bar, bar) || other.bar == bar));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,bar);
}

@override
String toString() {
    return 'CommandDto.addVoice(track: $track, bar: $bar)';
}


}

/// @nodoc
abstract mixin class $CommandDto_AddVoiceCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_AddVoiceCopyWith(CommandDto_AddVoice value, $Res Function(CommandDto_AddVoice) _then) = _$CommandDto_AddVoiceCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, int bar
});




}
/// @nodoc
class _$CommandDto_AddVoiceCopyWithImpl<$Res>
    implements $CommandDto_AddVoiceCopyWith<$Res> {
  _$CommandDto_AddVoiceCopyWithImpl(this._self, this._then);

  final CommandDto_AddVoice _self;
  final $Res Function(CommandDto_AddVoice) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? bar = null,}) {
  return _then(CommandDto_AddVoice(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,bar: null == bar ? _self.bar : bar // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class CommandDto_RemoveVoice extends CommandDto {
  const CommandDto_RemoveVoice({required this.track, required this.bar, required this.voice}): super._();
  

 final  PlatformInt64 track;
 final  int bar;
 final  int voice;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_RemoveVoiceCopyWith<CommandDto_RemoveVoice> get copyWith => _$CommandDto_RemoveVoiceCopyWithImpl<CommandDto_RemoveVoice>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_RemoveVoice&&(identical(other.track, track) || other.track == track)&&(identical(other.bar, bar) || other.bar == bar)&&(identical(other.voice, voice) || other.voice == voice));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,bar,voice);
}

@override
String toString() {
    return 'CommandDto.removeVoice(track: $track, bar: $bar, voice: $voice)';
}


}

/// @nodoc
abstract mixin class $CommandDto_RemoveVoiceCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_RemoveVoiceCopyWith(CommandDto_RemoveVoice value, $Res Function(CommandDto_RemoveVoice) _then) = _$CommandDto_RemoveVoiceCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, int bar, int voice
});




}
/// @nodoc
class _$CommandDto_RemoveVoiceCopyWithImpl<$Res>
    implements $CommandDto_RemoveVoiceCopyWith<$Res> {
  _$CommandDto_RemoveVoiceCopyWithImpl(this._self, this._then);

  final CommandDto_RemoveVoice _self;
  final $Res Function(CommandDto_RemoveVoice) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? bar = null,Object? voice = null,}) {
  return _then(CommandDto_RemoveVoice(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,bar: null == bar ? _self.bar : bar // ignore: cast_nullable_to_non_nullable
as int,voice: null == voice ? _self.voice : voice // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class CommandDto_ShiftFrets extends CommandDto {
  const CommandDto_ShiftFrets({required this.track, required this.fromBar, required this.toBar, required this.delta}): super._();
  

 final  PlatformInt64 track;
 final  int fromBar;
 final  int toBar;
 final  int delta;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_ShiftFretsCopyWith<CommandDto_ShiftFrets> get copyWith => _$CommandDto_ShiftFretsCopyWithImpl<CommandDto_ShiftFrets>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_ShiftFrets&&(identical(other.track, track) || other.track == track)&&(identical(other.fromBar, fromBar) || other.fromBar == fromBar)&&(identical(other.toBar, toBar) || other.toBar == toBar)&&(identical(other.delta, delta) || other.delta == delta));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,fromBar,toBar,delta);
}

@override
String toString() {
    return 'CommandDto.shiftFrets(track: $track, fromBar: $fromBar, toBar: $toBar, delta: $delta)';
}


}

/// @nodoc
abstract mixin class $CommandDto_ShiftFretsCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_ShiftFretsCopyWith(CommandDto_ShiftFrets value, $Res Function(CommandDto_ShiftFrets) _then) = _$CommandDto_ShiftFretsCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, int fromBar, int toBar, int delta
});




}
/// @nodoc
class _$CommandDto_ShiftFretsCopyWithImpl<$Res>
    implements $CommandDto_ShiftFretsCopyWith<$Res> {
  _$CommandDto_ShiftFretsCopyWithImpl(this._self, this._then);

  final CommandDto_ShiftFrets _self;
  final $Res Function(CommandDto_ShiftFrets) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? fromBar = null,Object? toBar = null,Object? delta = null,}) {
  return _then(CommandDto_ShiftFrets(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,fromBar: null == fromBar ? _self.fromBar : fromBar // ignore: cast_nullable_to_non_nullable
as int,toBar: null == toBar ? _self.toBar : toBar // ignore: cast_nullable_to_non_nullable
as int,delta: null == delta ? _self.delta : delta // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class CommandDto_AddTrack extends CommandDto {
  const CommandDto_AddTrack({required this.name, required this.instrument}): super._();
  

 final  String name;
 final  InstrumentDto instrument;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_AddTrackCopyWith<CommandDto_AddTrack> get copyWith => _$CommandDto_AddTrackCopyWithImpl<CommandDto_AddTrack>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_AddTrack&&(identical(other.name, name) || other.name == name)&&(identical(other.instrument, instrument) || other.instrument == instrument));
}


@override
int get hashCode {
    return Object.hash(runtimeType,name,instrument);
}

@override
String toString() {
    return 'CommandDto.addTrack(name: $name, instrument: $instrument)';
}


}

/// @nodoc
abstract mixin class $CommandDto_AddTrackCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_AddTrackCopyWith(CommandDto_AddTrack value, $Res Function(CommandDto_AddTrack) _then) = _$CommandDto_AddTrackCopyWithImpl;
@useResult
$Res call({
 String name, InstrumentDto instrument
});




}
/// @nodoc
class _$CommandDto_AddTrackCopyWithImpl<$Res>
    implements $CommandDto_AddTrackCopyWith<$Res> {
  _$CommandDto_AddTrackCopyWithImpl(this._self, this._then);

  final CommandDto_AddTrack _self;
  final $Res Function(CommandDto_AddTrack) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,Object? instrument = null,}) {
  return _then(CommandDto_AddTrack(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,instrument: null == instrument ? _self.instrument : instrument // ignore: cast_nullable_to_non_nullable
as InstrumentDto,
  ));
}


}

/// @nodoc


class CommandDto_RemoveTrack extends CommandDto {
  const CommandDto_RemoveTrack({required this.track}): super._();
  

 final  PlatformInt64 track;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_RemoveTrackCopyWith<CommandDto_RemoveTrack> get copyWith => _$CommandDto_RemoveTrackCopyWithImpl<CommandDto_RemoveTrack>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_RemoveTrack&&(identical(other.track, track) || other.track == track));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track);
}

@override
String toString() {
    return 'CommandDto.removeTrack(track: $track)';
}


}

/// @nodoc
abstract mixin class $CommandDto_RemoveTrackCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_RemoveTrackCopyWith(CommandDto_RemoveTrack value, $Res Function(CommandDto_RemoveTrack) _then) = _$CommandDto_RemoveTrackCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track
});




}
/// @nodoc
class _$CommandDto_RemoveTrackCopyWithImpl<$Res>
    implements $CommandDto_RemoveTrackCopyWith<$Res> {
  _$CommandDto_RemoveTrackCopyWithImpl(this._self, this._then);

  final CommandDto_RemoveTrack _self;
  final $Res Function(CommandDto_RemoveTrack) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,}) {
  return _then(CommandDto_RemoveTrack(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class CommandDto_RenameTrack extends CommandDto {
  const CommandDto_RenameTrack({required this.track, required this.name}): super._();
  

 final  PlatformInt64 track;
 final  String name;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_RenameTrackCopyWith<CommandDto_RenameTrack> get copyWith => _$CommandDto_RenameTrackCopyWithImpl<CommandDto_RenameTrack>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_RenameTrack&&(identical(other.track, track) || other.track == track)&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,name);
}

@override
String toString() {
    return 'CommandDto.renameTrack(track: $track, name: $name)';
}


}

/// @nodoc
abstract mixin class $CommandDto_RenameTrackCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_RenameTrackCopyWith(CommandDto_RenameTrack value, $Res Function(CommandDto_RenameTrack) _then) = _$CommandDto_RenameTrackCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, String name
});




}
/// @nodoc
class _$CommandDto_RenameTrackCopyWithImpl<$Res>
    implements $CommandDto_RenameTrackCopyWith<$Res> {
  _$CommandDto_RenameTrackCopyWithImpl(this._self, this._then);

  final CommandDto_RenameTrack _self;
  final $Res Function(CommandDto_RenameTrack) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? name = null,}) {
  return _then(CommandDto_RenameTrack(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class CommandDto_SetInstrument extends CommandDto {
  const CommandDto_SetInstrument({required this.track, required this.instrument}): super._();
  

 final  PlatformInt64 track;
 final  InstrumentDto instrument;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_SetInstrumentCopyWith<CommandDto_SetInstrument> get copyWith => _$CommandDto_SetInstrumentCopyWithImpl<CommandDto_SetInstrument>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_SetInstrument&&(identical(other.track, track) || other.track == track)&&(identical(other.instrument, instrument) || other.instrument == instrument));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,instrument);
}

@override
String toString() {
    return 'CommandDto.setInstrument(track: $track, instrument: $instrument)';
}


}

/// @nodoc
abstract mixin class $CommandDto_SetInstrumentCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_SetInstrumentCopyWith(CommandDto_SetInstrument value, $Res Function(CommandDto_SetInstrument) _then) = _$CommandDto_SetInstrumentCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, InstrumentDto instrument
});




}
/// @nodoc
class _$CommandDto_SetInstrumentCopyWithImpl<$Res>
    implements $CommandDto_SetInstrumentCopyWith<$Res> {
  _$CommandDto_SetInstrumentCopyWithImpl(this._self, this._then);

  final CommandDto_SetInstrument _self;
  final $Res Function(CommandDto_SetInstrument) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? instrument = null,}) {
  return _then(CommandDto_SetInstrument(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,instrument: null == instrument ? _self.instrument : instrument // ignore: cast_nullable_to_non_nullable
as InstrumentDto,
  ));
}


}

/// @nodoc


class CommandDto_MoveTrack extends CommandDto {
  const CommandDto_MoveTrack({required this.track, required this.to}): super._();
  

 final  PlatformInt64 track;
 final  int to;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_MoveTrackCopyWith<CommandDto_MoveTrack> get copyWith => _$CommandDto_MoveTrackCopyWithImpl<CommandDto_MoveTrack>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_MoveTrack&&(identical(other.track, track) || other.track == track)&&(identical(other.to, to) || other.to == to));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,to);
}

@override
String toString() {
    return 'CommandDto.moveTrack(track: $track, to: $to)';
}


}

/// @nodoc
abstract mixin class $CommandDto_MoveTrackCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_MoveTrackCopyWith(CommandDto_MoveTrack value, $Res Function(CommandDto_MoveTrack) _then) = _$CommandDto_MoveTrackCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, int to
});




}
/// @nodoc
class _$CommandDto_MoveTrackCopyWithImpl<$Res>
    implements $CommandDto_MoveTrackCopyWith<$Res> {
  _$CommandDto_MoveTrackCopyWithImpl(this._self, this._then);

  final CommandDto_MoveTrack _self;
  final $Res Function(CommandDto_MoveTrack) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? to = null,}) {
  return _then(CommandDto_MoveTrack(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,to: null == to ? _self.to : to // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class CommandDto_Retune extends CommandDto {
  const CommandDto_Retune({required this.track, required this.instrument, required this.keepPitches}): super._();
  

 final  PlatformInt64 track;
 final  InstrumentDto instrument;
 final  bool keepPitches;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_RetuneCopyWith<CommandDto_Retune> get copyWith => _$CommandDto_RetuneCopyWithImpl<CommandDto_Retune>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_Retune&&(identical(other.track, track) || other.track == track)&&(identical(other.instrument, instrument) || other.instrument == instrument)&&(identical(other.keepPitches, keepPitches) || other.keepPitches == keepPitches));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,instrument,keepPitches);
}

@override
String toString() {
    return 'CommandDto.retune(track: $track, instrument: $instrument, keepPitches: $keepPitches)';
}


}

/// @nodoc
abstract mixin class $CommandDto_RetuneCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_RetuneCopyWith(CommandDto_Retune value, $Res Function(CommandDto_Retune) _then) = _$CommandDto_RetuneCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, InstrumentDto instrument, bool keepPitches
});




}
/// @nodoc
class _$CommandDto_RetuneCopyWithImpl<$Res>
    implements $CommandDto_RetuneCopyWith<$Res> {
  _$CommandDto_RetuneCopyWithImpl(this._self, this._then);

  final CommandDto_Retune _self;
  final $Res Function(CommandDto_Retune) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? instrument = null,Object? keepPitches = null,}) {
  return _then(CommandDto_Retune(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,instrument: null == instrument ? _self.instrument : instrument // ignore: cast_nullable_to_non_nullable
as InstrumentDto,keepPitches: null == keepPitches ? _self.keepPitches : keepPitches // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class CommandDto_PasteBeats extends CommandDto {
  const CommandDto_PasteBeats({required this.track, required this.bar, required this.voice, required this.index, required  List<ClipBeatDto> beats}): _beats = beats,super._();
  

 final  PlatformInt64 track;
 final  int bar;
 final  int voice;
 final  int index;
 final  List<ClipBeatDto> _beats;
 List<ClipBeatDto> get beats {
  if (_beats is EqualUnmodifiableListView) return _beats;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_beats);
}


/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_PasteBeatsCopyWith<CommandDto_PasteBeats> get copyWith => _$CommandDto_PasteBeatsCopyWithImpl<CommandDto_PasteBeats>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_PasteBeats&&(identical(other.track, track) || other.track == track)&&(identical(other.bar, bar) || other.bar == bar)&&(identical(other.voice, voice) || other.voice == voice)&&(identical(other.index, index) || other.index == index)&&const DeepCollectionEquality().equals(other.beats, _beats));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,bar,voice,index,const DeepCollectionEquality().hash(_beats));
}

@override
String toString() {
    return 'CommandDto.pasteBeats(track: $track, bar: $bar, voice: $voice, index: $index, beats: $beats)';
}


}

/// @nodoc
abstract mixin class $CommandDto_PasteBeatsCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_PasteBeatsCopyWith(CommandDto_PasteBeats value, $Res Function(CommandDto_PasteBeats) _then) = _$CommandDto_PasteBeatsCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, int bar, int voice, int index, List<ClipBeatDto> beats
});




}
/// @nodoc
class _$CommandDto_PasteBeatsCopyWithImpl<$Res>
    implements $CommandDto_PasteBeatsCopyWith<$Res> {
  _$CommandDto_PasteBeatsCopyWithImpl(this._self, this._then);

  final CommandDto_PasteBeats _self;
  final $Res Function(CommandDto_PasteBeats) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? bar = null,Object? voice = null,Object? index = null,Object? beats = null,}) {
  return _then(CommandDto_PasteBeats(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,bar: null == bar ? _self.bar : bar // ignore: cast_nullable_to_non_nullable
as int,voice: null == voice ? _self.voice : voice // ignore: cast_nullable_to_non_nullable
as int,index: null == index ? _self.index : index // ignore: cast_nullable_to_non_nullable
as int,beats: null == beats ? _self._beats : beats // ignore: cast_nullable_to_non_nullable
as List<ClipBeatDto>,
  ));
}


}

/// @nodoc


class CommandDto_RemoveBeats extends CommandDto {
  const CommandDto_RemoveBeats({required this.track, required this.voice, required this.fromBar, required this.fromIndex, required this.toBar, required this.toIndex}): super._();
  

 final  PlatformInt64 track;
 final  int voice;
 final  int fromBar;
 final  int fromIndex;
 final  int toBar;
 final  int toIndex;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CommandDto_RemoveBeatsCopyWith<CommandDto_RemoveBeats> get copyWith => _$CommandDto_RemoveBeatsCopyWithImpl<CommandDto_RemoveBeats>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CommandDto_RemoveBeats&&(identical(other.track, track) || other.track == track)&&(identical(other.voice, voice) || other.voice == voice)&&(identical(other.fromBar, fromBar) || other.fromBar == fromBar)&&(identical(other.fromIndex, fromIndex) || other.fromIndex == fromIndex)&&(identical(other.toBar, toBar) || other.toBar == toBar)&&(identical(other.toIndex, toIndex) || other.toIndex == toIndex));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,voice,fromBar,fromIndex,toBar,toIndex);
}

@override
String toString() {
    return 'CommandDto.removeBeats(track: $track, voice: $voice, fromBar: $fromBar, fromIndex: $fromIndex, toBar: $toBar, toIndex: $toIndex)';
}


}

/// @nodoc
abstract mixin class $CommandDto_RemoveBeatsCopyWith<$Res> implements $CommandDtoCopyWith<$Res> {
  factory $CommandDto_RemoveBeatsCopyWith(CommandDto_RemoveBeats value, $Res Function(CommandDto_RemoveBeats) _then) = _$CommandDto_RemoveBeatsCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, int voice, int fromBar, int fromIndex, int toBar, int toIndex
});




}
/// @nodoc
class _$CommandDto_RemoveBeatsCopyWithImpl<$Res>
    implements $CommandDto_RemoveBeatsCopyWith<$Res> {
  _$CommandDto_RemoveBeatsCopyWithImpl(this._self, this._then);

  final CommandDto_RemoveBeats _self;
  final $Res Function(CommandDto_RemoveBeats) _then;

/// Create a copy of CommandDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? voice = null,Object? fromBar = null,Object? fromIndex = null,Object? toBar = null,Object? toIndex = null,}) {
  return _then(CommandDto_RemoveBeats(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,voice: null == voice ? _self.voice : voice // ignore: cast_nullable_to_non_nullable
as int,fromBar: null == fromBar ? _self.fromBar : fromBar // ignore: cast_nullable_to_non_nullable
as int,fromIndex: null == fromIndex ? _self.fromIndex : fromIndex // ignore: cast_nullable_to_non_nullable
as int,toBar: null == toBar ? _self.toBar : toBar // ignore: cast_nullable_to_non_nullable
as int,toIndex: null == toIndex ? _self.toIndex : toIndex // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc
mixin _$EventKindDto {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'EventKindDto()';
}


}

/// @nodoc
class $EventKindDtoCopyWith<$Res>  {
$EventKindDtoCopyWith(EventKindDto _, $Res Function(EventKindDto) __);
}


/// Adds pattern-matching-related methods to [EventKindDto].
extension EventKindDtoPatterns on EventKindDto {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( EventKindDto_TrackAdded value)?  trackAdded,TResult Function( EventKindDto_TrackRemoved value)?  trackRemoved,TResult Function( EventKindDto_TrackChanged value)?  trackChanged,TResult Function( EventKindDto_TracksReordered value)?  tracksReordered,TResult Function( EventKindDto_MasterBarsInserted value)?  masterBarsInserted,TResult Function( EventKindDto_MasterBarsRemoved value)?  masterBarsRemoved,TResult Function( EventKindDto_MasterBarsChanged value)?  masterBarsChanged,TResult Function( EventKindDto_BarContentChanged value)?  barContentChanged,TResult Function( EventKindDto_TempoMapChanged value)?  tempoMapChanged,TResult Function( EventKindDto_TiesRemoved value)?  tiesRemoved,TResult Function( EventKindDto_DocumentSaved value)?  documentSaved,TResult Function( EventKindDto_DirtyChanged value)?  dirtyChanged,TResult Function( EventKindDto_UndoAvailabilityChanged value)?  undoAvailabilityChanged,TResult Function( EventKindDto_HistoryApplied value)?  historyApplied,TResult Function( EventKindDto_PlaybackStateChanged value)?  playbackStateChanged,TResult Function( EventKindDto_SoloChanged value)?  soloChanged,TResult Function( EventKindDto_MixChanged value)?  mixChanged,TResult Function( EventKindDto_MuteChanged value)?  muteChanged,TResult Function( EventKindDto_TempoFactorChanged value)?  tempoFactorChanged,required TResult orElse(),}){
final _that = this;
switch (_that) {
case EventKindDto_TrackAdded() when trackAdded != null:
return trackAdded(_that);case EventKindDto_TrackRemoved() when trackRemoved != null:
return trackRemoved(_that);case EventKindDto_TrackChanged() when trackChanged != null:
return trackChanged(_that);case EventKindDto_TracksReordered() when tracksReordered != null:
return tracksReordered(_that);case EventKindDto_MasterBarsInserted() when masterBarsInserted != null:
return masterBarsInserted(_that);case EventKindDto_MasterBarsRemoved() when masterBarsRemoved != null:
return masterBarsRemoved(_that);case EventKindDto_MasterBarsChanged() when masterBarsChanged != null:
return masterBarsChanged(_that);case EventKindDto_BarContentChanged() when barContentChanged != null:
return barContentChanged(_that);case EventKindDto_TempoMapChanged() when tempoMapChanged != null:
return tempoMapChanged(_that);case EventKindDto_TiesRemoved() when tiesRemoved != null:
return tiesRemoved(_that);case EventKindDto_DocumentSaved() when documentSaved != null:
return documentSaved(_that);case EventKindDto_DirtyChanged() when dirtyChanged != null:
return dirtyChanged(_that);case EventKindDto_UndoAvailabilityChanged() when undoAvailabilityChanged != null:
return undoAvailabilityChanged(_that);case EventKindDto_HistoryApplied() when historyApplied != null:
return historyApplied(_that);case EventKindDto_PlaybackStateChanged() when playbackStateChanged != null:
return playbackStateChanged(_that);case EventKindDto_SoloChanged() when soloChanged != null:
return soloChanged(_that);case EventKindDto_MixChanged() when mixChanged != null:
return mixChanged(_that);case EventKindDto_MuteChanged() when muteChanged != null:
return muteChanged(_that);case EventKindDto_TempoFactorChanged() when tempoFactorChanged != null:
return tempoFactorChanged(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( EventKindDto_TrackAdded value)  trackAdded,required TResult Function( EventKindDto_TrackRemoved value)  trackRemoved,required TResult Function( EventKindDto_TrackChanged value)  trackChanged,required TResult Function( EventKindDto_TracksReordered value)  tracksReordered,required TResult Function( EventKindDto_MasterBarsInserted value)  masterBarsInserted,required TResult Function( EventKindDto_MasterBarsRemoved value)  masterBarsRemoved,required TResult Function( EventKindDto_MasterBarsChanged value)  masterBarsChanged,required TResult Function( EventKindDto_BarContentChanged value)  barContentChanged,required TResult Function( EventKindDto_TempoMapChanged value)  tempoMapChanged,required TResult Function( EventKindDto_TiesRemoved value)  tiesRemoved,required TResult Function( EventKindDto_DocumentSaved value)  documentSaved,required TResult Function( EventKindDto_DirtyChanged value)  dirtyChanged,required TResult Function( EventKindDto_UndoAvailabilityChanged value)  undoAvailabilityChanged,required TResult Function( EventKindDto_HistoryApplied value)  historyApplied,required TResult Function( EventKindDto_PlaybackStateChanged value)  playbackStateChanged,required TResult Function( EventKindDto_SoloChanged value)  soloChanged,required TResult Function( EventKindDto_MixChanged value)  mixChanged,required TResult Function( EventKindDto_MuteChanged value)  muteChanged,required TResult Function( EventKindDto_TempoFactorChanged value)  tempoFactorChanged,}){
final _that = this;
switch (_that) {
case EventKindDto_TrackAdded():
return trackAdded(_that);case EventKindDto_TrackRemoved():
return trackRemoved(_that);case EventKindDto_TrackChanged():
return trackChanged(_that);case EventKindDto_TracksReordered():
return tracksReordered(_that);case EventKindDto_MasterBarsInserted():
return masterBarsInserted(_that);case EventKindDto_MasterBarsRemoved():
return masterBarsRemoved(_that);case EventKindDto_MasterBarsChanged():
return masterBarsChanged(_that);case EventKindDto_BarContentChanged():
return barContentChanged(_that);case EventKindDto_TempoMapChanged():
return tempoMapChanged(_that);case EventKindDto_TiesRemoved():
return tiesRemoved(_that);case EventKindDto_DocumentSaved():
return documentSaved(_that);case EventKindDto_DirtyChanged():
return dirtyChanged(_that);case EventKindDto_UndoAvailabilityChanged():
return undoAvailabilityChanged(_that);case EventKindDto_HistoryApplied():
return historyApplied(_that);case EventKindDto_PlaybackStateChanged():
return playbackStateChanged(_that);case EventKindDto_SoloChanged():
return soloChanged(_that);case EventKindDto_MixChanged():
return mixChanged(_that);case EventKindDto_MuteChanged():
return muteChanged(_that);case EventKindDto_TempoFactorChanged():
return tempoFactorChanged(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( EventKindDto_TrackAdded value)?  trackAdded,TResult? Function( EventKindDto_TrackRemoved value)?  trackRemoved,TResult? Function( EventKindDto_TrackChanged value)?  trackChanged,TResult? Function( EventKindDto_TracksReordered value)?  tracksReordered,TResult? Function( EventKindDto_MasterBarsInserted value)?  masterBarsInserted,TResult? Function( EventKindDto_MasterBarsRemoved value)?  masterBarsRemoved,TResult? Function( EventKindDto_MasterBarsChanged value)?  masterBarsChanged,TResult? Function( EventKindDto_BarContentChanged value)?  barContentChanged,TResult? Function( EventKindDto_TempoMapChanged value)?  tempoMapChanged,TResult? Function( EventKindDto_TiesRemoved value)?  tiesRemoved,TResult? Function( EventKindDto_DocumentSaved value)?  documentSaved,TResult? Function( EventKindDto_DirtyChanged value)?  dirtyChanged,TResult? Function( EventKindDto_UndoAvailabilityChanged value)?  undoAvailabilityChanged,TResult? Function( EventKindDto_HistoryApplied value)?  historyApplied,TResult? Function( EventKindDto_PlaybackStateChanged value)?  playbackStateChanged,TResult? Function( EventKindDto_SoloChanged value)?  soloChanged,TResult? Function( EventKindDto_MixChanged value)?  mixChanged,TResult? Function( EventKindDto_MuteChanged value)?  muteChanged,TResult? Function( EventKindDto_TempoFactorChanged value)?  tempoFactorChanged,}){
final _that = this;
switch (_that) {
case EventKindDto_TrackAdded() when trackAdded != null:
return trackAdded(_that);case EventKindDto_TrackRemoved() when trackRemoved != null:
return trackRemoved(_that);case EventKindDto_TrackChanged() when trackChanged != null:
return trackChanged(_that);case EventKindDto_TracksReordered() when tracksReordered != null:
return tracksReordered(_that);case EventKindDto_MasterBarsInserted() when masterBarsInserted != null:
return masterBarsInserted(_that);case EventKindDto_MasterBarsRemoved() when masterBarsRemoved != null:
return masterBarsRemoved(_that);case EventKindDto_MasterBarsChanged() when masterBarsChanged != null:
return masterBarsChanged(_that);case EventKindDto_BarContentChanged() when barContentChanged != null:
return barContentChanged(_that);case EventKindDto_TempoMapChanged() when tempoMapChanged != null:
return tempoMapChanged(_that);case EventKindDto_TiesRemoved() when tiesRemoved != null:
return tiesRemoved(_that);case EventKindDto_DocumentSaved() when documentSaved != null:
return documentSaved(_that);case EventKindDto_DirtyChanged() when dirtyChanged != null:
return dirtyChanged(_that);case EventKindDto_UndoAvailabilityChanged() when undoAvailabilityChanged != null:
return undoAvailabilityChanged(_that);case EventKindDto_HistoryApplied() when historyApplied != null:
return historyApplied(_that);case EventKindDto_PlaybackStateChanged() when playbackStateChanged != null:
return playbackStateChanged(_that);case EventKindDto_SoloChanged() when soloChanged != null:
return soloChanged(_that);case EventKindDto_MixChanged() when mixChanged != null:
return mixChanged(_that);case EventKindDto_MuteChanged() when muteChanged != null:
return muteChanged(_that);case EventKindDto_TempoFactorChanged() when tempoFactorChanged != null:
return tempoFactorChanged(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( PlatformInt64 track)?  trackAdded,TResult Function( PlatformInt64 track)?  trackRemoved,TResult Function( PlatformInt64 track)?  trackChanged,TResult Function()?  tracksReordered,TResult Function( int at,  int count)?  masterBarsInserted,TResult Function( int at,  int count)?  masterBarsRemoved,TResult Function( int start,  int end)?  masterBarsChanged,TResult Function( PlatformInt64 track,  int start,  int end)?  barContentChanged,TResult Function()?  tempoMapChanged,TResult Function( PlatformInt64 track,  Int64List notes)?  tiesRemoved,TResult Function()?  documentSaved,TResult Function( bool dirty)?  dirtyChanged,TResult Function( bool canUndo,  bool canRedo)?  undoAvailabilityChanged,TResult Function()?  historyApplied,TResult Function( PlaybackStateDto state)?  playbackStateChanged,TResult Function( PlatformInt64 track,  bool soloed)?  soloChanged,TResult Function( PlatformInt64 track,  int volume,  int pan)?  mixChanged,TResult Function( PlatformInt64 track,  bool muted)?  muteChanged,TResult Function( int hundredths)?  tempoFactorChanged,required TResult orElse(),}) {final _that = this;
switch (_that) {
case EventKindDto_TrackAdded() when trackAdded != null:
return trackAdded(_that.track);case EventKindDto_TrackRemoved() when trackRemoved != null:
return trackRemoved(_that.track);case EventKindDto_TrackChanged() when trackChanged != null:
return trackChanged(_that.track);case EventKindDto_TracksReordered() when tracksReordered != null:
return tracksReordered();case EventKindDto_MasterBarsInserted() when masterBarsInserted != null:
return masterBarsInserted(_that.at,_that.count);case EventKindDto_MasterBarsRemoved() when masterBarsRemoved != null:
return masterBarsRemoved(_that.at,_that.count);case EventKindDto_MasterBarsChanged() when masterBarsChanged != null:
return masterBarsChanged(_that.start,_that.end);case EventKindDto_BarContentChanged() when barContentChanged != null:
return barContentChanged(_that.track,_that.start,_that.end);case EventKindDto_TempoMapChanged() when tempoMapChanged != null:
return tempoMapChanged();case EventKindDto_TiesRemoved() when tiesRemoved != null:
return tiesRemoved(_that.track,_that.notes);case EventKindDto_DocumentSaved() when documentSaved != null:
return documentSaved();case EventKindDto_DirtyChanged() when dirtyChanged != null:
return dirtyChanged(_that.dirty);case EventKindDto_UndoAvailabilityChanged() when undoAvailabilityChanged != null:
return undoAvailabilityChanged(_that.canUndo,_that.canRedo);case EventKindDto_HistoryApplied() when historyApplied != null:
return historyApplied();case EventKindDto_PlaybackStateChanged() when playbackStateChanged != null:
return playbackStateChanged(_that.state);case EventKindDto_SoloChanged() when soloChanged != null:
return soloChanged(_that.track,_that.soloed);case EventKindDto_MixChanged() when mixChanged != null:
return mixChanged(_that.track,_that.volume,_that.pan);case EventKindDto_MuteChanged() when muteChanged != null:
return muteChanged(_that.track,_that.muted);case EventKindDto_TempoFactorChanged() when tempoFactorChanged != null:
return tempoFactorChanged(_that.hundredths);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( PlatformInt64 track)  trackAdded,required TResult Function( PlatformInt64 track)  trackRemoved,required TResult Function( PlatformInt64 track)  trackChanged,required TResult Function()  tracksReordered,required TResult Function( int at,  int count)  masterBarsInserted,required TResult Function( int at,  int count)  masterBarsRemoved,required TResult Function( int start,  int end)  masterBarsChanged,required TResult Function( PlatformInt64 track,  int start,  int end)  barContentChanged,required TResult Function()  tempoMapChanged,required TResult Function( PlatformInt64 track,  Int64List notes)  tiesRemoved,required TResult Function()  documentSaved,required TResult Function( bool dirty)  dirtyChanged,required TResult Function( bool canUndo,  bool canRedo)  undoAvailabilityChanged,required TResult Function()  historyApplied,required TResult Function( PlaybackStateDto state)  playbackStateChanged,required TResult Function( PlatformInt64 track,  bool soloed)  soloChanged,required TResult Function( PlatformInt64 track,  int volume,  int pan)  mixChanged,required TResult Function( PlatformInt64 track,  bool muted)  muteChanged,required TResult Function( int hundredths)  tempoFactorChanged,}) {final _that = this;
switch (_that) {
case EventKindDto_TrackAdded():
return trackAdded(_that.track);case EventKindDto_TrackRemoved():
return trackRemoved(_that.track);case EventKindDto_TrackChanged():
return trackChanged(_that.track);case EventKindDto_TracksReordered():
return tracksReordered();case EventKindDto_MasterBarsInserted():
return masterBarsInserted(_that.at,_that.count);case EventKindDto_MasterBarsRemoved():
return masterBarsRemoved(_that.at,_that.count);case EventKindDto_MasterBarsChanged():
return masterBarsChanged(_that.start,_that.end);case EventKindDto_BarContentChanged():
return barContentChanged(_that.track,_that.start,_that.end);case EventKindDto_TempoMapChanged():
return tempoMapChanged();case EventKindDto_TiesRemoved():
return tiesRemoved(_that.track,_that.notes);case EventKindDto_DocumentSaved():
return documentSaved();case EventKindDto_DirtyChanged():
return dirtyChanged(_that.dirty);case EventKindDto_UndoAvailabilityChanged():
return undoAvailabilityChanged(_that.canUndo,_that.canRedo);case EventKindDto_HistoryApplied():
return historyApplied();case EventKindDto_PlaybackStateChanged():
return playbackStateChanged(_that.state);case EventKindDto_SoloChanged():
return soloChanged(_that.track,_that.soloed);case EventKindDto_MixChanged():
return mixChanged(_that.track,_that.volume,_that.pan);case EventKindDto_MuteChanged():
return muteChanged(_that.track,_that.muted);case EventKindDto_TempoFactorChanged():
return tempoFactorChanged(_that.hundredths);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( PlatformInt64 track)?  trackAdded,TResult? Function( PlatformInt64 track)?  trackRemoved,TResult? Function( PlatformInt64 track)?  trackChanged,TResult? Function()?  tracksReordered,TResult? Function( int at,  int count)?  masterBarsInserted,TResult? Function( int at,  int count)?  masterBarsRemoved,TResult? Function( int start,  int end)?  masterBarsChanged,TResult? Function( PlatformInt64 track,  int start,  int end)?  barContentChanged,TResult? Function()?  tempoMapChanged,TResult? Function( PlatformInt64 track,  Int64List notes)?  tiesRemoved,TResult? Function()?  documentSaved,TResult? Function( bool dirty)?  dirtyChanged,TResult? Function( bool canUndo,  bool canRedo)?  undoAvailabilityChanged,TResult? Function()?  historyApplied,TResult? Function( PlaybackStateDto state)?  playbackStateChanged,TResult? Function( PlatformInt64 track,  bool soloed)?  soloChanged,TResult? Function( PlatformInt64 track,  int volume,  int pan)?  mixChanged,TResult? Function( PlatformInt64 track,  bool muted)?  muteChanged,TResult? Function( int hundredths)?  tempoFactorChanged,}) {final _that = this;
switch (_that) {
case EventKindDto_TrackAdded() when trackAdded != null:
return trackAdded(_that.track);case EventKindDto_TrackRemoved() when trackRemoved != null:
return trackRemoved(_that.track);case EventKindDto_TrackChanged() when trackChanged != null:
return trackChanged(_that.track);case EventKindDto_TracksReordered() when tracksReordered != null:
return tracksReordered();case EventKindDto_MasterBarsInserted() when masterBarsInserted != null:
return masterBarsInserted(_that.at,_that.count);case EventKindDto_MasterBarsRemoved() when masterBarsRemoved != null:
return masterBarsRemoved(_that.at,_that.count);case EventKindDto_MasterBarsChanged() when masterBarsChanged != null:
return masterBarsChanged(_that.start,_that.end);case EventKindDto_BarContentChanged() when barContentChanged != null:
return barContentChanged(_that.track,_that.start,_that.end);case EventKindDto_TempoMapChanged() when tempoMapChanged != null:
return tempoMapChanged();case EventKindDto_TiesRemoved() when tiesRemoved != null:
return tiesRemoved(_that.track,_that.notes);case EventKindDto_DocumentSaved() when documentSaved != null:
return documentSaved();case EventKindDto_DirtyChanged() when dirtyChanged != null:
return dirtyChanged(_that.dirty);case EventKindDto_UndoAvailabilityChanged() when undoAvailabilityChanged != null:
return undoAvailabilityChanged(_that.canUndo,_that.canRedo);case EventKindDto_HistoryApplied() when historyApplied != null:
return historyApplied();case EventKindDto_PlaybackStateChanged() when playbackStateChanged != null:
return playbackStateChanged(_that.state);case EventKindDto_SoloChanged() when soloChanged != null:
return soloChanged(_that.track,_that.soloed);case EventKindDto_MixChanged() when mixChanged != null:
return mixChanged(_that.track,_that.volume,_that.pan);case EventKindDto_MuteChanged() when muteChanged != null:
return muteChanged(_that.track,_that.muted);case EventKindDto_TempoFactorChanged() when tempoFactorChanged != null:
return tempoFactorChanged(_that.hundredths);case _:
  return null;

}
}

}

/// @nodoc


class EventKindDto_TrackAdded extends EventKindDto {
  const EventKindDto_TrackAdded({required this.track}): super._();
  

 final  PlatformInt64 track;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_TrackAddedCopyWith<EventKindDto_TrackAdded> get copyWith => _$EventKindDto_TrackAddedCopyWithImpl<EventKindDto_TrackAdded>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_TrackAdded&&(identical(other.track, track) || other.track == track));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track);
}

@override
String toString() {
    return 'EventKindDto.trackAdded(track: $track)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_TrackAddedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_TrackAddedCopyWith(EventKindDto_TrackAdded value, $Res Function(EventKindDto_TrackAdded) _then) = _$EventKindDto_TrackAddedCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track
});




}
/// @nodoc
class _$EventKindDto_TrackAddedCopyWithImpl<$Res>
    implements $EventKindDto_TrackAddedCopyWith<$Res> {
  _$EventKindDto_TrackAddedCopyWithImpl(this._self, this._then);

  final EventKindDto_TrackAdded _self;
  final $Res Function(EventKindDto_TrackAdded) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,}) {
  return _then(EventKindDto_TrackAdded(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class EventKindDto_TrackRemoved extends EventKindDto {
  const EventKindDto_TrackRemoved({required this.track}): super._();
  

 final  PlatformInt64 track;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_TrackRemovedCopyWith<EventKindDto_TrackRemoved> get copyWith => _$EventKindDto_TrackRemovedCopyWithImpl<EventKindDto_TrackRemoved>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_TrackRemoved&&(identical(other.track, track) || other.track == track));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track);
}

@override
String toString() {
    return 'EventKindDto.trackRemoved(track: $track)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_TrackRemovedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_TrackRemovedCopyWith(EventKindDto_TrackRemoved value, $Res Function(EventKindDto_TrackRemoved) _then) = _$EventKindDto_TrackRemovedCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track
});




}
/// @nodoc
class _$EventKindDto_TrackRemovedCopyWithImpl<$Res>
    implements $EventKindDto_TrackRemovedCopyWith<$Res> {
  _$EventKindDto_TrackRemovedCopyWithImpl(this._self, this._then);

  final EventKindDto_TrackRemoved _self;
  final $Res Function(EventKindDto_TrackRemoved) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,}) {
  return _then(EventKindDto_TrackRemoved(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class EventKindDto_TrackChanged extends EventKindDto {
  const EventKindDto_TrackChanged({required this.track}): super._();
  

 final  PlatformInt64 track;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_TrackChangedCopyWith<EventKindDto_TrackChanged> get copyWith => _$EventKindDto_TrackChangedCopyWithImpl<EventKindDto_TrackChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_TrackChanged&&(identical(other.track, track) || other.track == track));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track);
}

@override
String toString() {
    return 'EventKindDto.trackChanged(track: $track)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_TrackChangedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_TrackChangedCopyWith(EventKindDto_TrackChanged value, $Res Function(EventKindDto_TrackChanged) _then) = _$EventKindDto_TrackChangedCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track
});




}
/// @nodoc
class _$EventKindDto_TrackChangedCopyWithImpl<$Res>
    implements $EventKindDto_TrackChangedCopyWith<$Res> {
  _$EventKindDto_TrackChangedCopyWithImpl(this._self, this._then);

  final EventKindDto_TrackChanged _self;
  final $Res Function(EventKindDto_TrackChanged) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,}) {
  return _then(EventKindDto_TrackChanged(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class EventKindDto_TracksReordered extends EventKindDto {
  const EventKindDto_TracksReordered(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_TracksReordered);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'EventKindDto.tracksReordered()';
}


}




/// @nodoc


class EventKindDto_MasterBarsInserted extends EventKindDto {
  const EventKindDto_MasterBarsInserted({required this.at, required this.count}): super._();
  

 final  int at;
 final  int count;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_MasterBarsInsertedCopyWith<EventKindDto_MasterBarsInserted> get copyWith => _$EventKindDto_MasterBarsInsertedCopyWithImpl<EventKindDto_MasterBarsInserted>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_MasterBarsInserted&&(identical(other.at, at) || other.at == at)&&(identical(other.count, count) || other.count == count));
}


@override
int get hashCode {
    return Object.hash(runtimeType,at,count);
}

@override
String toString() {
    return 'EventKindDto.masterBarsInserted(at: $at, count: $count)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_MasterBarsInsertedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_MasterBarsInsertedCopyWith(EventKindDto_MasterBarsInserted value, $Res Function(EventKindDto_MasterBarsInserted) _then) = _$EventKindDto_MasterBarsInsertedCopyWithImpl;
@useResult
$Res call({
 int at, int count
});




}
/// @nodoc
class _$EventKindDto_MasterBarsInsertedCopyWithImpl<$Res>
    implements $EventKindDto_MasterBarsInsertedCopyWith<$Res> {
  _$EventKindDto_MasterBarsInsertedCopyWithImpl(this._self, this._then);

  final EventKindDto_MasterBarsInserted _self;
  final $Res Function(EventKindDto_MasterBarsInserted) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? at = null,Object? count = null,}) {
  return _then(EventKindDto_MasterBarsInserted(
at: null == at ? _self.at : at // ignore: cast_nullable_to_non_nullable
as int,count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class EventKindDto_MasterBarsRemoved extends EventKindDto {
  const EventKindDto_MasterBarsRemoved({required this.at, required this.count}): super._();
  

 final  int at;
 final  int count;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_MasterBarsRemovedCopyWith<EventKindDto_MasterBarsRemoved> get copyWith => _$EventKindDto_MasterBarsRemovedCopyWithImpl<EventKindDto_MasterBarsRemoved>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_MasterBarsRemoved&&(identical(other.at, at) || other.at == at)&&(identical(other.count, count) || other.count == count));
}


@override
int get hashCode {
    return Object.hash(runtimeType,at,count);
}

@override
String toString() {
    return 'EventKindDto.masterBarsRemoved(at: $at, count: $count)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_MasterBarsRemovedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_MasterBarsRemovedCopyWith(EventKindDto_MasterBarsRemoved value, $Res Function(EventKindDto_MasterBarsRemoved) _then) = _$EventKindDto_MasterBarsRemovedCopyWithImpl;
@useResult
$Res call({
 int at, int count
});




}
/// @nodoc
class _$EventKindDto_MasterBarsRemovedCopyWithImpl<$Res>
    implements $EventKindDto_MasterBarsRemovedCopyWith<$Res> {
  _$EventKindDto_MasterBarsRemovedCopyWithImpl(this._self, this._then);

  final EventKindDto_MasterBarsRemoved _self;
  final $Res Function(EventKindDto_MasterBarsRemoved) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? at = null,Object? count = null,}) {
  return _then(EventKindDto_MasterBarsRemoved(
at: null == at ? _self.at : at // ignore: cast_nullable_to_non_nullable
as int,count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class EventKindDto_MasterBarsChanged extends EventKindDto {
  const EventKindDto_MasterBarsChanged({required this.start, required this.end}): super._();
  

 final  int start;
 final  int end;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_MasterBarsChangedCopyWith<EventKindDto_MasterBarsChanged> get copyWith => _$EventKindDto_MasterBarsChangedCopyWithImpl<EventKindDto_MasterBarsChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_MasterBarsChanged&&(identical(other.start, start) || other.start == start)&&(identical(other.end, end) || other.end == end));
}


@override
int get hashCode {
    return Object.hash(runtimeType,start,end);
}

@override
String toString() {
    return 'EventKindDto.masterBarsChanged(start: $start, end: $end)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_MasterBarsChangedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_MasterBarsChangedCopyWith(EventKindDto_MasterBarsChanged value, $Res Function(EventKindDto_MasterBarsChanged) _then) = _$EventKindDto_MasterBarsChangedCopyWithImpl;
@useResult
$Res call({
 int start, int end
});




}
/// @nodoc
class _$EventKindDto_MasterBarsChangedCopyWithImpl<$Res>
    implements $EventKindDto_MasterBarsChangedCopyWith<$Res> {
  _$EventKindDto_MasterBarsChangedCopyWithImpl(this._self, this._then);

  final EventKindDto_MasterBarsChanged _self;
  final $Res Function(EventKindDto_MasterBarsChanged) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? start = null,Object? end = null,}) {
  return _then(EventKindDto_MasterBarsChanged(
start: null == start ? _self.start : start // ignore: cast_nullable_to_non_nullable
as int,end: null == end ? _self.end : end // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class EventKindDto_BarContentChanged extends EventKindDto {
  const EventKindDto_BarContentChanged({required this.track, required this.start, required this.end}): super._();
  

 final  PlatformInt64 track;
 final  int start;
 final  int end;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_BarContentChangedCopyWith<EventKindDto_BarContentChanged> get copyWith => _$EventKindDto_BarContentChangedCopyWithImpl<EventKindDto_BarContentChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_BarContentChanged&&(identical(other.track, track) || other.track == track)&&(identical(other.start, start) || other.start == start)&&(identical(other.end, end) || other.end == end));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,start,end);
}

@override
String toString() {
    return 'EventKindDto.barContentChanged(track: $track, start: $start, end: $end)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_BarContentChangedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_BarContentChangedCopyWith(EventKindDto_BarContentChanged value, $Res Function(EventKindDto_BarContentChanged) _then) = _$EventKindDto_BarContentChangedCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, int start, int end
});




}
/// @nodoc
class _$EventKindDto_BarContentChangedCopyWithImpl<$Res>
    implements $EventKindDto_BarContentChangedCopyWith<$Res> {
  _$EventKindDto_BarContentChangedCopyWithImpl(this._self, this._then);

  final EventKindDto_BarContentChanged _self;
  final $Res Function(EventKindDto_BarContentChanged) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? start = null,Object? end = null,}) {
  return _then(EventKindDto_BarContentChanged(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,start: null == start ? _self.start : start // ignore: cast_nullable_to_non_nullable
as int,end: null == end ? _self.end : end // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class EventKindDto_TempoMapChanged extends EventKindDto {
  const EventKindDto_TempoMapChanged(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_TempoMapChanged);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'EventKindDto.tempoMapChanged()';
}


}




/// @nodoc


class EventKindDto_TiesRemoved extends EventKindDto {
  const EventKindDto_TiesRemoved({required this.track, required this.notes}): super._();
  

 final  PlatformInt64 track;
 final  Int64List notes;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_TiesRemovedCopyWith<EventKindDto_TiesRemoved> get copyWith => _$EventKindDto_TiesRemovedCopyWithImpl<EventKindDto_TiesRemoved>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_TiesRemoved&&(identical(other.track, track) || other.track == track)&&const DeepCollectionEquality().equals(other.notes, notes));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,const DeepCollectionEquality().hash(notes));
}

@override
String toString() {
    return 'EventKindDto.tiesRemoved(track: $track, notes: $notes)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_TiesRemovedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_TiesRemovedCopyWith(EventKindDto_TiesRemoved value, $Res Function(EventKindDto_TiesRemoved) _then) = _$EventKindDto_TiesRemovedCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, Int64List notes
});




}
/// @nodoc
class _$EventKindDto_TiesRemovedCopyWithImpl<$Res>
    implements $EventKindDto_TiesRemovedCopyWith<$Res> {
  _$EventKindDto_TiesRemovedCopyWithImpl(this._self, this._then);

  final EventKindDto_TiesRemoved _self;
  final $Res Function(EventKindDto_TiesRemoved) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? notes = null,}) {
  return _then(EventKindDto_TiesRemoved(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,notes: null == notes ? _self.notes : notes // ignore: cast_nullable_to_non_nullable
as Int64List,
  ));
}


}

/// @nodoc


class EventKindDto_DocumentSaved extends EventKindDto {
  const EventKindDto_DocumentSaved(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_DocumentSaved);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'EventKindDto.documentSaved()';
}


}




/// @nodoc


class EventKindDto_DirtyChanged extends EventKindDto {
  const EventKindDto_DirtyChanged({required this.dirty}): super._();
  

 final  bool dirty;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_DirtyChangedCopyWith<EventKindDto_DirtyChanged> get copyWith => _$EventKindDto_DirtyChangedCopyWithImpl<EventKindDto_DirtyChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_DirtyChanged&&(identical(other.dirty, dirty) || other.dirty == dirty));
}


@override
int get hashCode {
    return Object.hash(runtimeType,dirty);
}

@override
String toString() {
    return 'EventKindDto.dirtyChanged(dirty: $dirty)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_DirtyChangedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_DirtyChangedCopyWith(EventKindDto_DirtyChanged value, $Res Function(EventKindDto_DirtyChanged) _then) = _$EventKindDto_DirtyChangedCopyWithImpl;
@useResult
$Res call({
 bool dirty
});




}
/// @nodoc
class _$EventKindDto_DirtyChangedCopyWithImpl<$Res>
    implements $EventKindDto_DirtyChangedCopyWith<$Res> {
  _$EventKindDto_DirtyChangedCopyWithImpl(this._self, this._then);

  final EventKindDto_DirtyChanged _self;
  final $Res Function(EventKindDto_DirtyChanged) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? dirty = null,}) {
  return _then(EventKindDto_DirtyChanged(
dirty: null == dirty ? _self.dirty : dirty // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class EventKindDto_UndoAvailabilityChanged extends EventKindDto {
  const EventKindDto_UndoAvailabilityChanged({required this.canUndo, required this.canRedo}): super._();
  

 final  bool canUndo;
 final  bool canRedo;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_UndoAvailabilityChangedCopyWith<EventKindDto_UndoAvailabilityChanged> get copyWith => _$EventKindDto_UndoAvailabilityChangedCopyWithImpl<EventKindDto_UndoAvailabilityChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_UndoAvailabilityChanged&&(identical(other.canUndo, canUndo) || other.canUndo == canUndo)&&(identical(other.canRedo, canRedo) || other.canRedo == canRedo));
}


@override
int get hashCode {
    return Object.hash(runtimeType,canUndo,canRedo);
}

@override
String toString() {
    return 'EventKindDto.undoAvailabilityChanged(canUndo: $canUndo, canRedo: $canRedo)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_UndoAvailabilityChangedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_UndoAvailabilityChangedCopyWith(EventKindDto_UndoAvailabilityChanged value, $Res Function(EventKindDto_UndoAvailabilityChanged) _then) = _$EventKindDto_UndoAvailabilityChangedCopyWithImpl;
@useResult
$Res call({
 bool canUndo, bool canRedo
});




}
/// @nodoc
class _$EventKindDto_UndoAvailabilityChangedCopyWithImpl<$Res>
    implements $EventKindDto_UndoAvailabilityChangedCopyWith<$Res> {
  _$EventKindDto_UndoAvailabilityChangedCopyWithImpl(this._self, this._then);

  final EventKindDto_UndoAvailabilityChanged _self;
  final $Res Function(EventKindDto_UndoAvailabilityChanged) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? canUndo = null,Object? canRedo = null,}) {
  return _then(EventKindDto_UndoAvailabilityChanged(
canUndo: null == canUndo ? _self.canUndo : canUndo // ignore: cast_nullable_to_non_nullable
as bool,canRedo: null == canRedo ? _self.canRedo : canRedo // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class EventKindDto_HistoryApplied extends EventKindDto {
  const EventKindDto_HistoryApplied(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_HistoryApplied);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'EventKindDto.historyApplied()';
}


}




/// @nodoc


class EventKindDto_PlaybackStateChanged extends EventKindDto {
  const EventKindDto_PlaybackStateChanged({required this.state}): super._();
  

 final  PlaybackStateDto state;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_PlaybackStateChangedCopyWith<EventKindDto_PlaybackStateChanged> get copyWith => _$EventKindDto_PlaybackStateChangedCopyWithImpl<EventKindDto_PlaybackStateChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_PlaybackStateChanged&&(identical(other.state, state) || other.state == state));
}


@override
int get hashCode {
    return Object.hash(runtimeType,state);
}

@override
String toString() {
    return 'EventKindDto.playbackStateChanged(state: $state)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_PlaybackStateChangedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_PlaybackStateChangedCopyWith(EventKindDto_PlaybackStateChanged value, $Res Function(EventKindDto_PlaybackStateChanged) _then) = _$EventKindDto_PlaybackStateChangedCopyWithImpl;
@useResult
$Res call({
 PlaybackStateDto state
});




}
/// @nodoc
class _$EventKindDto_PlaybackStateChangedCopyWithImpl<$Res>
    implements $EventKindDto_PlaybackStateChangedCopyWith<$Res> {
  _$EventKindDto_PlaybackStateChangedCopyWithImpl(this._self, this._then);

  final EventKindDto_PlaybackStateChanged _self;
  final $Res Function(EventKindDto_PlaybackStateChanged) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? state = null,}) {
  return _then(EventKindDto_PlaybackStateChanged(
state: null == state ? _self.state : state // ignore: cast_nullable_to_non_nullable
as PlaybackStateDto,
  ));
}


}

/// @nodoc


class EventKindDto_SoloChanged extends EventKindDto {
  const EventKindDto_SoloChanged({required this.track, required this.soloed}): super._();
  

 final  PlatformInt64 track;
 final  bool soloed;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_SoloChangedCopyWith<EventKindDto_SoloChanged> get copyWith => _$EventKindDto_SoloChangedCopyWithImpl<EventKindDto_SoloChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_SoloChanged&&(identical(other.track, track) || other.track == track)&&(identical(other.soloed, soloed) || other.soloed == soloed));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,soloed);
}

@override
String toString() {
    return 'EventKindDto.soloChanged(track: $track, soloed: $soloed)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_SoloChangedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_SoloChangedCopyWith(EventKindDto_SoloChanged value, $Res Function(EventKindDto_SoloChanged) _then) = _$EventKindDto_SoloChangedCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, bool soloed
});




}
/// @nodoc
class _$EventKindDto_SoloChangedCopyWithImpl<$Res>
    implements $EventKindDto_SoloChangedCopyWith<$Res> {
  _$EventKindDto_SoloChangedCopyWithImpl(this._self, this._then);

  final EventKindDto_SoloChanged _self;
  final $Res Function(EventKindDto_SoloChanged) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? soloed = null,}) {
  return _then(EventKindDto_SoloChanged(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,soloed: null == soloed ? _self.soloed : soloed // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class EventKindDto_MixChanged extends EventKindDto {
  const EventKindDto_MixChanged({required this.track, required this.volume, required this.pan}): super._();
  

 final  PlatformInt64 track;
 final  int volume;
 final  int pan;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_MixChangedCopyWith<EventKindDto_MixChanged> get copyWith => _$EventKindDto_MixChangedCopyWithImpl<EventKindDto_MixChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_MixChanged&&(identical(other.track, track) || other.track == track)&&(identical(other.volume, volume) || other.volume == volume)&&(identical(other.pan, pan) || other.pan == pan));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,volume,pan);
}

@override
String toString() {
    return 'EventKindDto.mixChanged(track: $track, volume: $volume, pan: $pan)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_MixChangedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_MixChangedCopyWith(EventKindDto_MixChanged value, $Res Function(EventKindDto_MixChanged) _then) = _$EventKindDto_MixChangedCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, int volume, int pan
});




}
/// @nodoc
class _$EventKindDto_MixChangedCopyWithImpl<$Res>
    implements $EventKindDto_MixChangedCopyWith<$Res> {
  _$EventKindDto_MixChangedCopyWithImpl(this._self, this._then);

  final EventKindDto_MixChanged _self;
  final $Res Function(EventKindDto_MixChanged) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? volume = null,Object? pan = null,}) {
  return _then(EventKindDto_MixChanged(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,volume: null == volume ? _self.volume : volume // ignore: cast_nullable_to_non_nullable
as int,pan: null == pan ? _self.pan : pan // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class EventKindDto_MuteChanged extends EventKindDto {
  const EventKindDto_MuteChanged({required this.track, required this.muted}): super._();
  

 final  PlatformInt64 track;
 final  bool muted;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_MuteChangedCopyWith<EventKindDto_MuteChanged> get copyWith => _$EventKindDto_MuteChangedCopyWithImpl<EventKindDto_MuteChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_MuteChanged&&(identical(other.track, track) || other.track == track)&&(identical(other.muted, muted) || other.muted == muted));
}


@override
int get hashCode {
    return Object.hash(runtimeType,track,muted);
}

@override
String toString() {
    return 'EventKindDto.muteChanged(track: $track, muted: $muted)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_MuteChangedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_MuteChangedCopyWith(EventKindDto_MuteChanged value, $Res Function(EventKindDto_MuteChanged) _then) = _$EventKindDto_MuteChangedCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 track, bool muted
});




}
/// @nodoc
class _$EventKindDto_MuteChangedCopyWithImpl<$Res>
    implements $EventKindDto_MuteChangedCopyWith<$Res> {
  _$EventKindDto_MuteChangedCopyWithImpl(this._self, this._then);

  final EventKindDto_MuteChanged _self;
  final $Res Function(EventKindDto_MuteChanged) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? track = null,Object? muted = null,}) {
  return _then(EventKindDto_MuteChanged(
track: null == track ? _self.track : track // ignore: cast_nullable_to_non_nullable
as PlatformInt64,muted: null == muted ? _self.muted : muted // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class EventKindDto_TempoFactorChanged extends EventKindDto {
  const EventKindDto_TempoFactorChanged({required this.hundredths}): super._();
  

 final  int hundredths;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EventKindDto_TempoFactorChangedCopyWith<EventKindDto_TempoFactorChanged> get copyWith => _$EventKindDto_TempoFactorChangedCopyWithImpl<EventKindDto_TempoFactorChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EventKindDto_TempoFactorChanged&&(identical(other.hundredths, hundredths) || other.hundredths == hundredths));
}


@override
int get hashCode {
    return Object.hash(runtimeType,hundredths);
}

@override
String toString() {
    return 'EventKindDto.tempoFactorChanged(hundredths: $hundredths)';
}


}

/// @nodoc
abstract mixin class $EventKindDto_TempoFactorChangedCopyWith<$Res> implements $EventKindDtoCopyWith<$Res> {
  factory $EventKindDto_TempoFactorChangedCopyWith(EventKindDto_TempoFactorChanged value, $Res Function(EventKindDto_TempoFactorChanged) _then) = _$EventKindDto_TempoFactorChangedCopyWithImpl;
@useResult
$Res call({
 int hundredths
});




}
/// @nodoc
class _$EventKindDto_TempoFactorChangedCopyWithImpl<$Res>
    implements $EventKindDto_TempoFactorChangedCopyWith<$Res> {
  _$EventKindDto_TempoFactorChangedCopyWithImpl(this._self, this._then);

  final EventKindDto_TempoFactorChanged _self;
  final $Res Function(EventKindDto_TempoFactorChanged) _then;

/// Create a copy of EventKindDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? hundredths = null,}) {
  return _then(EventKindDto_TempoFactorChanged(
hundredths: null == hundredths ? _self.hundredths : hundredths // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

// dart format on
