"""Regression checks for exercise placement and preserved book links."""
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

from doclinks import anchors
import sync_questions
import check_excerpts


def question(title, topic, exercise=None):
    text = f'### {title}\n\n<!-- chapters: {topic} -->\n\n'
    if exercise:
        text += f'<!-- exercise: {exercise} -->\n\n'
    text += '**Level:** Expert\n\nPredict a result.\n\n'
    text += '<details>\n<summary>Answer</summary>\n\nAn explanation.\n\n</details>'
    return title, [topic], text


class ExerciseTests(unittest.TestCase):
    def test_topic_match_does_not_assign_homework(self):
        bank = {level: [] for level in sync_questions.LEVELS}
        bank['newcomer'] = [question('First observation', 'capture', 'capture')]
        bank['expert'] = [question('Advanced follow-up', 'capture')]
        self.assertEqual(
            [q[0] for q in sync_questions.selection('capture', bank)],
            ['First observation'],
        )

    def test_expert_exercise_can_be_explicitly_selected(self):
        bank = {level: [] for level in sync_questions.LEVELS}
        bank['expert'] = [question('Changed requirements', 'capstone', 'capstone')]
        self.assertEqual(len(sync_questions.selection('capstone', bank)), 1)
        self.assertEqual(sync_questions.selection('capture', bank), [])

    def test_regeneration_preserves_prose_answers_and_old_question_links(self):
        bank = {level: [] for level in sync_questions.LEVELS}
        bank['newcomer'] = [question('First observation', 'capture', 'capture')]
        bank['expert'] = [question('Advanced follow-up', 'capture')]
        with TemporaryDirectory() as directory:
            path = Path(directory) / 'capture.md'
            path.write_text('# Lesson\n\nA concrete situation.\n')
            result = sync_questions.expected(path, bank)
            path.write_text(result)
            self.assertEqual(sync_questions.expected(path, bank), result)
        self.assertTrue(result.startswith('# Lesson\n\nA concrete situation.\n'))
        self.assertIn('<summary>Answer</summary>', result)
        self.assertNotIn('### Advanced follow-up', result)
        self.assertIn('advanced-follow-up', anchors(result))
        self.assertIn('../questions/expert.md#advanced-follow-up', result)
        self.assertNotIn('<!-- exercise:', result)
        self.assertNotIn('<!-- chapters:', result)
        self.assertNotIn('**Level:**', result)
        self.assertNotIn('\n\n\n', result)


class AnchorTests(unittest.TestCase):
    def test_explicit_aliases_and_repeated_headings(self):
        self.assertEqual(
            anchors('<a id="old-heading"></a>\n# New heading\n# New heading'),
            {'old-heading', 'new-heading', 'new-heading-1'},
        )

    def test_example_html_is_not_a_real_anchor(self):
        text = '```html\n<a id="example"></a>\n```\n`<a id="inline"></a>`'
        self.assertEqual(anchors(text), set())


class ExcerptTests(unittest.TestCase):
    def test_historical_code_requires_a_bug_or_performance_reason(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'example.rs').write_text('fixed();\n')
            page = root / 'lesson.md'
            excerpt = '<!-- excerpt: example.rs#L1-L1 @123abcd -->\n```rs\nbroken();\n```\n'
            with patch.object(check_excerpts, 'historical_lines', return_value=['broken();']) as read:
                for reason in ('', '<!-- historical: refactor; moved a helper -->\n',
                               '<!-- historical: bug;  -->\n'):
                    with self.subTest(reason=reason):
                        page.write_text(reason + excerpt)
                        self.assertIn('bug or performance reason', check_excerpts.check(root, page)[0])
                for kind in ('bug', 'performance'):
                    page.write_text(f'<!-- historical: {kind}; compare the earlier loop with its correction -->\n' + excerpt)
                    self.assertEqual(check_excerpts.check(root, page), [])
                read.assert_called_with(root, '123abcd', 'example.rs')

    def test_current_code_cannot_keep_a_historical_label(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'example.rs').write_text('fixed();\n')
            page = root / 'lesson.md'
            page.write_text('<!-- historical: bug; the earlier implementation -->\n'
                            '<!-- excerpt: example.rs#L1-L1 @current -->\n```rs\nfixed();\n```\n')
            self.assertIn('must not have a historical label', check_excerpts.check(root, page)[0])

    def test_current_source_changes_are_detected(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'example.rs'
            source.write_text('fn answer() {\n    let value = 1;\n}\n')
            page = root / 'lesson.md'
            page.write_text('<!-- excerpt: example.rs#L1-L3 @current -->\n```rs\nfn answer() {\n    let value = 1;\n}\n```\n')
            self.assertEqual(check_excerpts.check(root, page), [])
            source.write_text('fn answer() {\n    let value = 2;\n}\n')
            self.assertIn('line not found', check_excerpts.check(root, page)[0])

    def test_cuts_must_be_declared_and_markers_must_be_valid(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'example.rs').write_text('first\nmiddle\nlast\n')
            page = root / 'lesson.md'
            page.write_text('<!-- excerpt: example.rs#L1-L3 @current -->\n```rs\nfirst\nlast\n```\n')
            self.assertIn('without a cut', check_excerpts.check(root, page)[0])
            page.write_text(page.read_text().replace('first\nlast', 'first\n// … middle …\nlast'))
            self.assertEqual(check_excerpts.check(root, page), [])
            page.write_text(page.read_text().replace('@current', '@typo'))
            self.assertIn('malformed', check_excerpts.check(root, page)[0])


class TerminalCellsTests(unittest.TestCase):
    def test_supplied_dimensions_retain_the_last_cell(self):
        from terminal_cells import screen_text
        screen = screen_text(b'\x1b[34;120HX', 120, 34)
        self.assertEqual(screen.splitlines()[33][119], 'X')
        self.assertEqual(len(screen.splitlines()), 34)


if __name__ == '__main__':
    unittest.main()
