// S10 fixture generator (throwaway, not part of the product).
//
// Builds the 14 fixture songs of README section 4 with the TuxGuitar 2.0.1 song model
// and writes them with TuxGuitar's own GP3, GP4 and GP5 writers, then reads each file
// back with TuxGuitar's GP readers. Only the public API of the TuxGuitar jars is used
// (class and method signatures listed with `javap -public`); no TuxGuitar source code
// was read. The generated files are the evidence; this program is not shipped.
//
// Build and run: see ../README.md section 4.

import app.tuxguitar.io.base.TGSongReaderHandle;
import app.tuxguitar.io.base.TGSongStreamContext;
import app.tuxguitar.io.base.TGSongWriterHandle;
import app.tuxguitar.io.gtp.*;
import app.tuxguitar.song.factory.TGFactory;
import app.tuxguitar.song.managers.TGSongManager;
import app.tuxguitar.song.models.*;
import app.tuxguitar.song.models.effects.*;

import java.io.*;
import java.util.*;

public class MakeFixtures {
    static final TGFactory F = new TGFactory();
    static final TGSongManager M = new TGSongManager(F);
    static final long Q = TGDuration.QUARTER_TIME;

    // ------------------------------------------------------------- building

    /** A new song as File > New makes it, with `bars` empty bars. */
    static TGSong song(int bars) {
        TGSong s = M.newSong();
        while (s.countMeasureHeaders() < bars) M.addNewMeasureBeforeEnd(s);
        for (int t = 0; t < s.countTracks(); t++) clear(s.getTrack(t));
        return s;
    }

    static void clear(TGTrack track) {
        for (int m = 0; m < track.countMeasures(); m++) {
            TGMeasure measure = track.getMeasure(m);
            for (TGBeat b : new ArrayList<>(measure.getBeats())) measure.removeBeat(b);
        }
    }

    static TGDuration dur(int value, boolean dotted, int tupletEnters) {
        TGDuration d = F.newDuration();
        d.setValue(value);
        d.setDotted(dotted);
        if (tupletEnters > 0) {
            d.getDivision().setEnters(tupletEnters);
            d.getDivision().setTimes(2);
        }
        return d;
    }

    static TGDuration dur(int value) { return dur(value, false, 0); }

    /** The beat of `measure` at `offset` ticks from the bar start (created if missing). */
    static TGBeat beatAt(TGMeasure measure, long offset) {
        long start = measure.getStart() + offset;
        for (TGBeat b : measure.getBeats()) if (b.getStart() == start) return b;
        TGBeat b = F.newBeat();
        b.setStart(start);
        // TuxGuitar 2.0 keeps a second, precise time base next to the tick start.
        b.setPreciseStart(TGDuration.toPreciseTime(start));
        measure.addBeat(b);
        return b;
    }

    /** A voice of a beat with this duration and these notes (string, fret); no notes is a rest. */
    static TGVoice put(TGMeasure measure, long offset, int voice, TGDuration d, int[]... notes) {
        TGBeat b = beatAt(measure, offset);
        TGVoice v = b.getVoice(voice);
        v.setDuration(d);
        v.setEmpty(false);
        for (int[] n : notes) {
            TGNote note = F.newNote();
            note.setString(n[0]);
            note.setValue(n[1]);
            if (note.getEffect() == null) note.setEffect(F.newEffect());
            v.addNote(note);
        }
        return v;
    }

    static TGNoteEffect fx(TGVoice v) { return v.getNote(0).getEffect(); }

    static void fill(TGSong s) {
        for (int t = 0; t < s.countTracks(); t++) {
            TGTrack track = s.getTrack(t);
            for (int m = 0; m < track.countMeasures(); m++) {
                M.getMeasureManager().autoCompleteSilences(track.getMeasure(m));
            }
        }
    }

    static void tuning(TGTrack track, int... pitchesFromString1) {
        List<TGString> strings = new ArrayList<>();
        for (int i = 0; i < pitchesFromString1.length; i++) {
            TGString s = F.newString();
            s.setNumber(i + 1);
            s.setValue(pitchesFromString1[i]);
            strings.add(s);
        }
        track.setStrings(strings);
    }

    static void timeSignature(TGSong s, int bar, int numerator, int denominator) {
        TGTimeSignature ts = F.newTimeSignature();
        ts.setNumerator(numerator);
        ts.setDenominator(dur(denominator));
        M.changeTimeSignature(s, s.getMeasureHeader(bar), ts, true);
    }

    static final int[] STANDARD = {64, 59, 55, 50, 45, 40};
    static final int[][] C_CHORD = {{5, 3}, {4, 2}, {3, 0}, {2, 1}, {1, 0}};

    // ------------------------------------------------------------- fixtures

    static TGSong f01() { return song(1); }

    static TGSong f02() {
        TGSong s = song(1);
        put(s.getTrack(0).getMeasure(0), 0, 0, dur(4), new int[] {1, 0});
        fill(s);
        return s;
    }

    static TGSong f03() {
        TGSong s = song(1);
        put(s.getTrack(0).getMeasure(0), 0, 0, dur(4), C_CHORD);
        fill(s);
        return s;
    }

    static TGSong f04() {
        TGSong s = song(2);
        TGMeasure a = s.getTrack(0).getMeasure(0);
        put(a, 0, 0, dur(2), new int[] {1, 0});
        put(a, 2 * Q, 0, dur(4), new int[] {1, 1});
        put(a, 3 * Q, 0, dur(8), new int[] {1, 2});
        put(a, 3 * Q + Q / 2, 0, dur(16), new int[] {1, 3});
        put(a, 3 * Q + 3 * Q / 4, 0, dur(16), new int[] {1, 5});
        TGMeasure b = s.getTrack(0).getMeasure(1);
        put(b, 0, 0, dur(4, true, 0), new int[] {2, 0});
        put(b, 3 * Q / 2, 0, dur(8), new int[] {2, 1});
        put(b, 2 * Q, 0, dur(8, false, 3), new int[] {2, 3});
        put(b, 2 * Q + Q / 3, 0, dur(8, false, 3), new int[] {2, 5});
        put(b, 2 * Q + 2 * Q / 3, 0, dur(8, false, 3), new int[] {2, 7});
        put(b, 3 * Q, 0, dur(4));
        fill(s);
        return s;
    }

    static TGSong f05() {
        TGSong s = song(3);
        TGMeasure a = s.getTrack(0).getMeasure(0);
        put(a, 0, 0, dur(4), new int[] {2, 5});
        fx(put(a, Q, 0, dur(4), new int[] {2, 5})).setAccentuatedNote(true);
        fx(put(a, 2 * Q, 0, dur(4), new int[] {2, 5})).setHeavyAccentuatedNote(true);
        fx(put(a, 3 * Q, 0, dur(4), new int[] {2, 5})).setDeadNote(true);
        TGMeasure b = s.getTrack(0).getMeasure(1);
        fx(put(b, 0, 0, dur(4), new int[] {2, 5})).setPalmMute(true);
        fx(put(b, Q, 0, dur(4), new int[] {2, 5})).setLetRing(true);
        fx(put(b, 2 * Q, 0, dur(4), new int[] {2, 5})).setGhostNote(true);
        fx(put(b, 3 * Q, 0, dur(4), new int[] {2, 5})).setStaccato(true);
        TGMeasure c = s.getTrack(0).getMeasure(2);
        put(c, 0, 0, dur(2), new int[] {2, 5});
        put(c, 2 * Q, 0, dur(2), new int[] {2, 5}).getNote(0).setTiedNote(true);
        fill(s);
        return s;
    }

    static TGSong f06() {
        TGSong s = song(1);
        TGMeasure a = s.getTrack(0).getMeasure(0);
        put(a, 0, 0, dur(4), C_CHORD);
        beatAt(a, 0).getPickStroke().setDirection(TGPickStroke.PICK_STROKE_DOWN);
        put(a, Q, 0, dur(4), C_CHORD);
        beatAt(a, Q).getPickStroke().setDirection(TGPickStroke.PICK_STROKE_UP);
        put(a, 2 * Q, 0, dur(4), C_CHORD);
        beatAt(a, 2 * Q).getStroke().setDirection(TGStroke.STROKE_DOWN);
        beatAt(a, 2 * Q).getStroke().setValue(TGDuration.SIXTEENTH);
        put(a, 3 * Q, 0, dur(4), C_CHORD);
        fill(s);
        return s;
    }

    static TGSong f07() {
        TGSong s = song(2);
        TGTrack lead = s.getTrack(0);
        lead.setName("Lead");
        TGTrack rhythm = M.addTrack(s);
        rhythm.setName("Rhythm");
        tuning(rhythm, 64, 59, 55, 50, 45, 38);
        rhythm.setOffset(2);
        clear(rhythm);
        for (int bar = 0; bar < 2; bar++) {
            put(lead.getMeasure(bar), 0, 0, dur(4), new int[] {1, bar + 1});
            put(rhythm.getMeasure(bar), 0, 0, dur(4), new int[] {6, 0});
        }
        fill(s);
        return s;
    }

    static TGSong f08() {
        TGSong s = song(3);
        timeSignature(s, 1, 3, 4);
        timeSignature(s, 2, 6, 8);
        s.getMeasureHeader(0).getTempo().setQuarterValue(90);
        s.getMeasureHeader(1).getTempo().setQuarterValue(90);
        s.getMeasureHeader(2).getTempo().setQuarterValue(140);
        TGTrack t = s.getTrack(0);
        for (int bar = 0; bar < 3; bar++) {
            t.getMeasure(bar).setKeySignature(1); // G major
            put(t.getMeasure(bar), 0, 0, dur(4), new int[] {3, 0});
        }
        fill(s);
        return s;
    }

    static TGSong f09() {
        TGSong s = song(6);
        s.getMeasureHeader(1).setRepeatOpen(true);
        s.getMeasureHeader(3).setRepeatClose(1);
        s.getMeasureHeader(3).setRepeatAlternative(1);
        s.getMeasureHeader(4).setRepeatAlternative(1 << 1);
        TGMarker marker = F.newMarker();
        marker.setMeasure(2);
        marker.setTitle("Chorus");
        marker.setColor(F.newColor());
        M.updateMarker(s, marker);
        TGTrack t = s.getTrack(0);
        for (int bar = 0; bar < 6; bar++) put(t.getMeasure(bar), 0, 0, dur(1), new int[] {1, bar});
        fill(s);
        return s;
    }

    static TGSong f10() {
        TGSong s = song(1);
        TGMeasure a = s.getTrack(0).getMeasure(0);
        for (int k = 0; k < 4; k++) put(a, k * Q, 0, dur(4), new int[] {1, k});
        put(a, 0, 1, dur(2), new int[] {6, 3});
        put(a, 2 * Q, 1, dur(2));
        fill(s);
        return s;
    }

    static TGSong f11() {
        TGSong s = song(1);
        s.setName("Fixture text");
        s.setArtist("Tester");
        s.setAlbum("Bài hát");
        put(s.getTrack(0).getMeasure(0), 0, 0, dur(4), new int[] {1, 0});
        fill(s);
        return s;
    }

    static TGSong f12() {
        TGSong s = song(2);
        TGMeasure a = s.getTrack(0).getMeasure(0);
        TGEffectBend bend = F.newEffectBend();
        bend.addPoint(0, 0);
        bend.addPoint(TGEffectBend.MAX_POSITION_LENGTH / 2, 2 * TGEffectBend.SEMITONE_LENGTH);
        bend.addPoint(TGEffectBend.MAX_POSITION_LENGTH, 2 * TGEffectBend.SEMITONE_LENGTH);
        fx(put(a, 0, 0, dur(4), new int[] {3, 7})).setBend(bend);
        fx(put(a, Q, 0, dur(4), new int[] {3, 7})).setSlide(true);
        fx(put(a, 2 * Q, 0, dur(4), new int[] {3, 9})).setHammer(true);
        TGEffectHarmonic harmonic = F.newEffectHarmonic();
        harmonic.setType(TGEffectHarmonic.TYPE_NATURAL);
        fx(put(a, 3 * Q, 0, dur(4), new int[] {3, 12})).setHarmonic(harmonic);
        TGMeasure b = s.getTrack(0).getMeasure(1);
        fx(put(b, 0, 0, dur(4), new int[] {2, 8})).setVibrato(true);
        TGEffectGrace grace = F.newEffectGrace();
        grace.setFret(6);
        grace.setDuration(TGEffectGrace.DURATION_THIRTY_SECOND);
        grace.setDynamic(95);
        grace.setTransition(TGEffectGrace.TRANSITION_NONE);
        fx(put(b, Q, 0, dur(4), new int[] {2, 8})).setGrace(grace);
        TGEffectTremoloPicking tremolo = F.newEffectTremoloPicking();
        tremolo.setDuration(dur(8));
        fx(put(b, 2 * Q, 0, dur(4), new int[] {2, 8})).setTremoloPicking(tremolo);
        fx(put(b, 3 * Q, 0, dur(4), new int[] {2, 12})).setTapping(true);
        fill(s);
        return s;
    }

    static TGSong f13() {
        TGSong s = song(1);
        TGTrack t = s.getTrack(0);
        t.setName("Seven strings");
        tuning(t, 64, 59, 55, 50, 45, 40, 35);
        put(t.getMeasure(0), 0, 0, dur(4), new int[] {7, 0});
        fill(s);
        return s;
    }

    static TGSong f14() {
        TGSong s = song(1);
        TGTrack t = s.getTrack(0);
        TGChannel drums = M.addPercussionChannel(s);
        t.setChannelId(drums.getChannelId());
        t.setName("Drums");
        TGMeasure a = t.getMeasure(0);
        put(a, 0, 0, dur(4), new int[] {5, 36}); // kick
        put(a, Q, 0, dur(4), new int[] {3, 38}); // snare
        put(a, 2 * Q, 0, dur(4), new int[] {5, 36});
        put(a, 3 * Q, 0, dur(4), new int[] {3, 38}, new int[] {1, 42}); // snare and hi-hat
        fill(s);
        return s;
    }

    /** A chord diagram on a beat (very common in real files). */
    static TGSong f15() {
        TGSong s = song(1);
        TGMeasure a = s.getTrack(0).getMeasure(0);
        put(a, 0, 0, dur(4), C_CHORD);
        TGChord chord = F.newChord(6);
        chord.setName("C");
        chord.setFirstFret(0);
        int[] frets = {0, 1, 0, 2, 3, -1}; // string 1 first
        for (int i = 0; i < 6; i++) chord.addFretValue(i, frets[i]);
        beatAt(a, 0).setChord(chord);
        fill(s);
        return s;
    }

    /** A text on a beat. */
    static TGSong f16() {
        TGSong s = song(1);
        TGMeasure a = s.getTrack(0).getMeasure(0);
        put(a, 0, 0, dur(4), new int[] {1, 0});
        TGText text = F.newText();
        text.setValue("Intro");
        beatAt(a, 0).setText(text);
        fill(s);
        return s;
    }

    /** One bar that is the ending of passes 1 and 2, then the ending of pass 3. */
    static TGSong f17() {
        TGSong s = song(4);
        s.getMeasureHeader(0).setRepeatOpen(true);
        s.getMeasureHeader(1).setRepeatClose(2);
        s.getMeasureHeader(1).setRepeatAlternative(1 | 2);
        s.getMeasureHeader(2).setRepeatAlternative(4);
        TGTrack t = s.getTrack(0);
        for (int bar = 0; bar < 4; bar++) put(t.getMeasure(bar), 0, 0, dur(1), new int[] {1, bar});
        fill(s);
        return s;
    }

    /** Lyrics on the first track. */
    static TGSong f18() {
        TGSong s = song(1);
        TGTrack t = s.getTrack(0);
        for (int k = 0; k < 4; k++) put(t.getMeasure(0), k * Q, 0, dur(4), new int[] {1, k});
        TGLyric lyric = F.newLyric();
        lyric.setFrom(1);
        lyric.setLyrics("Một hai ba bốn");
        t.setLyrics(lyric);
        fill(s);
        return s;
    }

    // ------------------------------------------------------------- writing

    static GTPOutputStream writer(String ext, GTPSettings settings) {
        switch (ext) {
            case "gp5": return new GP5OutputStream(settings);
            case "gp4": return new GP4OutputStream(settings);
            default: return new GP3OutputStream(settings);
        }
    }

    static GTPInputStream reader(String ext, GTPSettings settings) {
        switch (ext) {
            case "gp5": return new GP5InputStream(settings);
            case "gp4": return new GP4InputStream(settings);
            default: return new GP3InputStream(settings);
        }
    }

    static void write(TGSong song, File file, String ext) throws Exception {
        GTPSettings settings = new GTPSettings();
        GTPOutputStream out = writer(ext, settings);
        out.init(F);
        try (OutputStream os = new FileOutputStream(file)) {
            TGSongWriterHandle h = new TGSongWriterHandle();
            h.setFactory(F);
            h.setSong(song);
            h.setOutputStream(os);
            h.setContext(new TGSongStreamContext());
            h.setFormat(out.getFileFormat());
            out.write(h);
        }
    }

    /** Reads the file back with TuxGuitar's reader and describes what it holds. */
    static String readBack(File file, String ext) throws Exception {
        GTPSettings settings = new GTPSettings();
        GTPInputStream in = reader(ext, settings);
        in.init(F);
        try (InputStream is = new FileInputStream(file)) {
            TGSongReaderHandle h = new TGSongReaderHandle();
            h.setFactory(F);
            h.setInputStream(is);
            h.setContext(new TGSongStreamContext());
            in.read(h);
            TGSong s = h.getSong();
            int notes = 0;
            for (int t = 0; t < s.countTracks(); t++) {
                TGTrack track = s.getTrack(t);
                for (int m = 0; m < track.countMeasures(); m++) {
                    for (TGBeat b : track.getMeasure(m).getBeats()) {
                        for (int v = 0; v < b.countVoices(); v++) notes += b.getVoice(v).countNotes();
                    }
                }
            }
            return "tracks=" + s.countTracks() + " bars=" + s.countMeasureHeaders() + " notes=" + notes;
        }
    }

    public static void main(String[] args) throws Exception {
        File dir = new File(args[0]);
        dir.mkdirs();
        Map<String, java.util.function.Supplier<TGSong>> fixtures = new LinkedHashMap<>();
        fixtures.put("f01-empty", MakeFixtures::f01);
        fixtures.put("f02-one-note", MakeFixtures::f02);
        fixtures.put("f03-chord", MakeFixtures::f03);
        fixtures.put("f04-durations", MakeFixtures::f04);
        fixtures.put("f05-note-effects", MakeFixtures::f05);
        fixtures.put("f06-strokes", MakeFixtures::f06);
        fixtures.put("f07-tracks", MakeFixtures::f07);
        fixtures.put("f08-meter-key-tempo", MakeFixtures::f08);
        fixtures.put("f09-repeats", MakeFixtures::f09);
        fixtures.put("f10-two-voices", MakeFixtures::f10);
        fixtures.put("f11-text", MakeFixtures::f11);
        fixtures.put("f12-unsupported", MakeFixtures::f12);
        fixtures.put("f13-seven-string", MakeFixtures::f13);
        fixtures.put("f14-percussion", MakeFixtures::f14);
        fixtures.put("f15-chord-diagram", MakeFixtures::f15);
        fixtures.put("f16-beat-text", MakeFixtures::f16);
        fixtures.put("f17-combined-endings", MakeFixtures::f17);
        fixtures.put("f18-lyrics", MakeFixtures::f18);
        for (Map.Entry<String, java.util.function.Supplier<TGSong>> e : fixtures.entrySet()) {
            for (String ext : new String[] {"gp5", "gp4", "gp3"}) {
                File file = new File(dir, e.getKey() + "." + ext);
                try {
                    write(e.getValue().get(), file, ext);
                    System.out.println(file.getName() + " " + file.length() + " bytes; read back: " + readBack(file, ext));
                } catch (Exception ex) {
                    System.out.println(file.getName() + " FAILED: " + ex);
                    file.delete();
                }
            }
        }
    }
}
