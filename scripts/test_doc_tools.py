"""Regression checks for exercise placement and preserved book links."""
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from doclinks import anchors
import sync_questions


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


if __name__ == '__main__':
    unittest.main()
