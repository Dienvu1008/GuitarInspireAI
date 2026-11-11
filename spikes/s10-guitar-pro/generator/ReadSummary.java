// S10 check (throwaway): reads GP3/GP4/GP5 files with TuxGuitar 2.0.1's own readers
// and prints each voice in the same text form as the spike reader's summary
// ("bar N voice V: 4 (1/0 acc) | 4 rest"), so the two can be compared line by line.
// Only the public API of the TuxGuitar jars is used.

import app.tuxguitar.io.base.TGSongReaderHandle;
import app.tuxguitar.io.base.TGSongStreamContext;
import app.tuxguitar.io.gtp.*;
import app.tuxguitar.song.factory.TGFactory;
import app.tuxguitar.song.models.*;
import app.tuxguitar.song.models.effects.*;

import java.io.*;
import java.util.*;

public class ReadSummary {
    static final TGFactory F = new TGFactory();

    static String dur(TGDuration d) {
        String s = String.valueOf(d.getValue());
        if (d.isDotted()) s += ".";
        if (d.getDivision().getEnters() != 1) s += "t" + d.getDivision().getEnters();
        return s;
    }

    static String note(TGNote n) {
        TGNoteEffect e = n.getEffect();
        StringBuilder s = new StringBuilder(n.getString() + "/" + n.getValue());
        if (n.isTiedNote()) s.append(" tie");
        if (e.isDeadNote()) s.append(" dead");
        if (e.isAccentuatedNote()) s.append(" acc");
        if (e.isHeavyAccentuatedNote()) s.append(" heavy");
        if (e.isGhostNote()) s.append(" ghost");
        if (e.isPalmMute()) s.append(" pm");
        if (e.isLetRing()) s.append(" ring");
        if (e.isStaccato()) s.append(" stacc");
        return s.toString();
    }

    public static void main(String[] args) throws Exception {
        for (String path : args) {
            String ext = path.substring(path.lastIndexOf('.') + 1);
            GTPSettings settings = new GTPSettings();
            GTPInputStream in = ext.equals("gp5") ? new GP5InputStream(settings)
                : ext.equals("gp4") ? new GP4InputStream(settings) : new GP3InputStream(settings);
            in.init(F);
            System.out.println(new File(path).getName());
            try (InputStream is = new FileInputStream(path)) {
                TGSongReaderHandle h = new TGSongReaderHandle();
                h.setFactory(F);
                h.setInputStream(is);
                h.setContext(new TGSongStreamContext());
                in.read(h);
                TGSong s = h.getSong();
                for (int m = 0; m < s.countMeasureHeaders(); m++) {
                    TGMeasureHeader mh = s.getMeasureHeader(m);
                    if (mh.getRepeatAlternative() != 0 || mh.getRepeatClose() != 0)
                        System.out.println("  extra bar " + (m + 1) + ": alternative " + mh.getRepeatAlternative() + " close " + mh.getRepeatClose());
                }
                for (int t = 0; t < s.countTracks(); t++) {
                    TGTrack track = s.getTrack(t);
                    System.out.println("  track " + (t + 1) + " \"" + track.getName() + "\" strings " + track.stringCount() + " capo " + track.getOffset());
                    String lyrics = track.getLyrics().getLyrics();
                    if (lyrics != null && !lyrics.isEmpty())
                        System.out.println("  extra lyrics from bar " + track.getLyrics().getFrom() + ": " + lyrics);
                    for (int m = 0; m < track.countMeasures(); m++) {
                        for (TGBeat b : track.getMeasure(m).getBeats()) {
                            if (b.getChord() != null) {
                                TGChord c = b.getChord();
                                List<String> fr = new ArrayList<>();
                                for (int i = 0; i < c.countStrings(); i++) fr.add(String.valueOf(c.getFretValue(i)));
                                System.out.println("  extra bar " + (m + 1) + " chord \"" + c.getName() + "\" fret " + c.getFirstFret() + " " + fr);
                            }
                            if (b.getText() != null)
                                System.out.println("  extra bar " + (m + 1) + " text \"" + b.getText().getValue() + "\"");
                        }
                    }
                    for (int m = 0; m < track.countMeasures(); m++) {
                        TGMeasure measure = track.getMeasure(m);
                        for (int v = 0; v < TGBeat.MAX_VOICES; v++) {
                            List<String> beats = new ArrayList<>();
                            for (TGBeat b : measure.getBeats()) {
                                TGVoice voice = b.getVoice(v);
                                if (voice.isEmpty()) continue;
                                String x = dur(voice.getDuration());
                                // The pick stroke belongs to the whole beat in TuxGuitar's
                                // model; it is printed on voice 1 only.
                                int pick = b.getPickStroke().getDirection();
                                if (v == 0 && pick == TGPickStroke.PICK_STROKE_DOWN) x += " down";
                                if (v == 0 && pick == TGPickStroke.PICK_STROKE_UP) x += " up";
                                if (voice.isRestVoice()) x += " rest";
                                List<TGNote> notes = new ArrayList<>(voice.getNotes());
                                notes.sort(Comparator.comparingInt(TGNote::getString));
                                List<String> ns = new ArrayList<>();
                                for (TGNote n : notes) ns.add(note(n));
                                if (!ns.isEmpty()) x += " (" + String.join(", ", ns) + ")";
                                beats.add(x);
                            }
                            if (!beats.isEmpty()) System.out.println("    bar " + (m + 1) + " voice " + (v + 1) + ": " + String.join(" | ", beats));
                        }
                    }
                }
            } catch (Exception e) {
                System.out.println("  FAILED: " + e);
            }
        }
    }
}
