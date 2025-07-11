use std::io;
use crossterm::event::{self, Event as CEvent, KeyEvent};
use ratatui::DefaultTerminal;
use std::time::{Duration, Instant};

use crate::ui::tui::ui::render_app;
use crate::utils;

use std::fs::OpenOptions;
use std::io::Write;


#[derive(PartialEq, Eq)]
pub enum GameState {
    NotStarted,
    Started,
    Results,
}

pub struct App {
    pub exit: bool,
    pub reference: String,
    pub pressed_vec: Vec<char>,
    pub pos1: usize,
    pub words_done: usize,
    pub is_correct: Vec<i32>,
    pub errors_this_second: f32,
    pub test_time: f32,
    pub start_time: Option<Instant>,
    pub game_state: GameState,
    pub config: bool,
    pub punctuation: bool,
    pub numbers: bool,
    pub time_mode: bool,
    pub word_mode: bool,
    pub quote: bool,
    pub batch_size: usize,
    pub selected_config: &'static str,
    pub speed_per_second: Vec<f64>,
    pub char_number: usize,
    pub errors_per_second: Vec<f32>,
    pub tab_pressed: bool,
    pub correct_count: usize,
    pub error_count: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            exit: false,
            reference: String::new(),
            pressed_vec: Vec::new(),
            pos1: 0,
            words_done: 0,
            is_correct: Vec::new(),
            errors_this_second: 0.0,
            test_time: 5.0,
            start_time: None,
            game_state: GameState::NotStarted,
            config: false,
            punctuation: false,
            numbers: false,
            time_mode: true,
            word_mode: false,
            quote: false,
            batch_size: 1,
            selected_config: "time",
            speed_per_second: Vec::new(),
            char_number: 0,
            errors_per_second: Vec::new(),
            tab_pressed: false,
            correct_count: 0,
            error_count: 0,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let word_list = utils::read_first_n_words(500);
        self.reference = utils::get_reference(false, false, &word_list, self.batch_size);
        self.is_correct = vec![0; self.reference.chars().count()];
        let reference_chars: Vec<char> = self.reference.chars().collect();
        let mut last_recorded_time = Instant::now();
        
        while !self.exit {
            if self.game_state != GameState::Started {
                last_recorded_time = Instant::now();
            }
            if event::poll(Duration::from_millis(16))? {
                if let CEvent::Key(key) = event::read()? {
                    self.handle_key_event(key, &reference_chars)?;
                }
            }
            let timer = if let Some(start_time) = self.start_time {
                if self.game_state == GameState::Started {
                    //println!("Y");
                    Instant::now().duration_since(start_time)
                } else {
                    Duration::from_secs(0)
                }
            } else {
                Duration::from_secs(0)
            };
            if self.test_time - (timer.as_secs_f32()) < 0.0 {
                if self.game_state == GameState::Started {
                    self.errors_per_second.push(self.errors_this_second);
                    self.game_state = GameState::Results;
                }
            }
            let now = Instant::now();
            let time_since_last = now.duration_since(last_recorded_time);

            if time_since_last >= Duration::from_secs(1) && self.game_state == GameState::Started && self.game_state != GameState::Results {
                let total_typed = self.pressed_vec.len();
                let chars_in_this_second = total_typed.saturating_sub(self.char_number);
                let cpm = chars_in_this_second as f64 * 60.0;

                self.speed_per_second.push(cpm);
                //println!("Y");

                self.char_number = total_typed;

                self.errors_per_second.push(self.errors_this_second);
                self.errors_this_second = 0.0;
                last_recorded_time += Duration::from_secs(1);
            }
            terminal.draw(|frame| render_app(frame, self, timer))?;
        }
        Ok(())
    }

    pub fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        reference_chars: &[char],
    ) -> io::Result<()> {
        use crossterm::event::KeyCode;

        let button_states = vec![
            ("! punctuation", self.punctuation, !self.quote),
            ("# numbers", self.numbers, !self.quote),
            ("|", true, true),
            ("time", self.time_mode, true),
            ("words", self.word_mode, true),
            ("quote", self.quote, true),
            ("|", true, true),
            ("15", self.test_time == 15.0, self.time_mode),
            ("30", self.test_time == 30.0, self.time_mode),
            ("60", self.test_time == 60.0, self.time_mode),
            ("120", self.test_time == 120.0, self.time_mode),
            ("25", self.batch_size == 25, self.word_mode),
            ("50", self.batch_size == 50, self.word_mode),
            ("100", self.batch_size == 100, self.word_mode),
        ];

        if key_event.kind == crossterm::event::KeyEventKind::Press {
            match key_event.code {
                KeyCode::Esc => self.exit = true,
                KeyCode::Backspace => {
                    if !self.pressed_vec.is_empty() && reference_chars.get(self.pos1) == Some(&' ') {
                        self.words_done = self.words_done.saturating_sub(1);
                    }
                    if self.is_correct[self.pos1] == 2 || self.is_correct[self.pos1] == 1 {
                        self.correct_count -= 1;
                    } else if self.is_correct[self.pos1] == -1 {
                        self.error_count -= 1;
                    }
                    self.pressed_vec.pop();
                    if self.pos1 > 0 {
                        self.pos1 -= 1;
                    }
                    self.config = false;
                }
                KeyCode::Up => {
                    if self.game_state != GameState::Results {
                        self.config = true;
                    }
                }
                KeyCode::Down => {
                    self.config = false;
                }
                KeyCode::Tab => {
                    self.tab_pressed = true;
                },
                KeyCode::Enter => {
                    if self.tab_pressed {
                        self.reference = utils::get_reference(self.punctuation, self.numbers, &utils::read_first_n_words(500), self.batch_size);
                        self.is_correct = vec![0; self.reference.chars().count()];
                        self.pressed_vec.clear();
                        self.pos1 = 0;
                        self.words_done = 0;
                        self.errors_this_second = 0.0;
                        self.start_time = None;
                        self.game_state = GameState::NotStarted;
                        self.speed_per_second.clear();
                        self.char_number = 0;
                        self.errors_per_second.clear();
                        self.tab_pressed = false;
                        self.correct_count = 0;
                        self.error_count = 0;
                    }
                    if self.config {
                        match self.selected_config {
                            "time" => {
                                self.time_mode = true;
                                self.word_mode = false;
                                self.quote = false;
                                self.batch_size = 50;
                            }
                            "words" => {
                                self.time_mode = false;
                                self.word_mode = true;
                                self.quote = false;
                            }
                            "quote" => {
                                self.quote = true;
                                self.time_mode = false;
                                self.word_mode = false;
                            }
                            "! punctuation" => {
                                self.punctuation = !self.punctuation;
                            }
                            "# numbers" => {
                                self.numbers = !self.numbers;
                            }
                            "15" => {
                                self.test_time = 15.0;
                            }
                            "30" => {
                                self.test_time = 30.0;
                            }
                            "60" => {
                                self.test_time = 60.0;
                            }
                            "120" => {
                                self.test_time = 120.0;
                            }
                            "25" => {
                                self.batch_size = 25;
                            }
                            "50" => {
                                self.batch_size = 50;
                            }
                            "100" => {
                                self.batch_size = 100;
                            }
                            _ => {}
                        }
                        if self.selected_config == "quote" {
                            self.reference = utils::get_random_quote();
                        }
                        else {
                            self.reference = utils::get_reference(self.punctuation, self.numbers, &utils::read_first_n_words(500), self.batch_size);

                        }
                        self.is_correct = vec![0; self.reference.chars().count()];
                        self.pressed_vec.clear();
                        self.pos1 = 0;
                        self.words_done = 0;
                        self.errors_this_second = 0.0;
                        self.start_time = None;
                        self.game_state = GameState::NotStarted;
                        self.speed_per_second.clear();
                        self.char_number = 0;
                        self.errors_per_second.clear();
                        self.tab_pressed = false;
                        self.correct_count = 0;
                        self.error_count = 0;
                    }
                }
                KeyCode::Left => {
                    if !self.config {
                        return Ok(());
                    }
                    for (i, (label, _state_val, visible)) in button_states.iter().enumerate() {
                        if *visible && self.selected_config == *label {
                            let start_index = i;
                            let mut j = if i == 0 {
                                button_states.len() - 1
                            } else {
                                i - 1
                            };

                            while j != start_index {
                                if button_states[j].2 && button_states[j].0 != "|" {
                                    self.selected_config = button_states[j].0;
                                    break;
                                }
                                j = if j == 0 {
                                    button_states.len() - 1
                                } else {
                                    j - 1
                                };
                            }
                            break;
                        }
                    }
                }
                KeyCode::Right => {
                    if !self.config {
                        return Ok(());
                    }
                    for (i, (label, _state_val, visible)) in button_states.iter().enumerate() {
                        if *visible && self.selected_config == *label {
                            if i == button_states.len() - 1 {
                                self.selected_config = button_states[0].0;
                            } else {
                                let mut next = i + 1;
                                if button_states[next].0 == "|" {
                                    next += 1;
                                }
                                while next != i {
                                    if next >= button_states.len() {
                                        next = 0;
                                    }
                                    if button_states[next].2 {
                                        self.selected_config = button_states[next].0;
                                        break;
                                    }
                                    next += 1;
                                }
                            }
                            break;
                        }
                    }
                }
                KeyCode::Char(ch) => {
                    if self.is_correct[0] == 0 && ch == ' ' {
                        return Ok(());
                    }
                    let reference_chars: Vec<char> = self.reference.chars().collect();
                    if let Some(&ref_char) = reference_chars.get(self.pos1) {
                        if self.game_state == GameState::Results {
                            return Ok(());
                        }
                        if self.game_state == GameState::NotStarted {
                            self.game_state = GameState::Started;
                            self.start_time = Some(Instant::now());
                        }
                        if self.is_correct.len() > self.pos1 {
                            
                            if ref_char == ch && self.is_correct[self.pos1] != -1 && self.is_correct[self.pos1] != 1 {
                                self.is_correct[self.pos1] = 2; // Correct
                                self.correct_count += 1;
                            } else if ref_char == ch && self.is_correct[self.pos1] == -1 {
                                self.is_correct[self.pos1] = 1; // Corrected
                            } else {
                                self.is_correct[self.pos1] = -1; // Incorrect
                                self.errors_this_second += 1.0;
                                self.error_count += 1;
                            }
                        }
                        self.pos1 += 1;
                        self.pressed_vec.push(ch);
                        if reference_chars.get(self.pos1) == Some(&' ') {
                            self.words_done += 1;
                        }
                    }
                    self.config = false;

                    if self.pos1 >= self.reference.chars().count() {
                        let mut file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("lposition.log")
                            .unwrap();
                        writeln!(file, "words_done: {}", self.words_done).unwrap();

                        self.words_done += 1;
                        self.reference = utils::get_reference(self.punctuation, self.numbers, &utils::read_first_n_words(500), self.batch_size);
                        self.is_correct = vec![0; self.reference.chars().count()];
                        self.pos1 = 0;

                        let mut file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("lposition.log")
                            .unwrap();
                        writeln!(file, "words_done: {}", self.words_done).unwrap();
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
