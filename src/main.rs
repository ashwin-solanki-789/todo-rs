use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::ops::{Add, Mul};
use std::{cmp, env, process};
// use std::os::unix::process;

use ncurses::*;

const REGULAR_PAIR: i16 = 0;
const HIGHLIGHT_PAIR: i16 = 1;

type Id = usize;

#[derive(Default, Copy, Clone)]
struct Demision {
    x: i32,
    y: i32
}
 
impl Add for Demision {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y
        }
    }
}

impl Mul for Demision {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self{
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y
        }
    }
}

impl Demision {
    fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y
        }
    }
}

enum LayoutKind  {
    Vert,
    Horz
}

struct Layout {
    kind: LayoutKind,
    pos: Demision,
    size: Demision,
}

impl Layout {
    // fn new(kind: LayoutKind, pos: Demision) -> Self {
    //     Self { kind, pos, size: Demision::new(0,0) }
    // }

    fn get_pos(&self) -> Demision {
        use LayoutKind::*;
        match self.kind {
            Horz => self.pos + self.size * Demision::new(1,0),
            Vert => self.pos + self.size * Demision::new(0,1),
        }
    }

    fn add_widget(&mut self, size: Demision) {
        use LayoutKind::*;
        match self.kind {
            Horz => {
                self.size.x += size.x;
                self.size.y = cmp::max(self.size.y, size.y);
            },
            Vert => {
                self.size.x = cmp::max(self.size.x, size.x);
                self.size.y += size.y;
            },
        }
    }
}

#[derive(Default)]
struct Ui {
    // list_curr: Option<Id>,
    layouts: Vec<Layout>
}

impl Ui {

    fn begin(&mut self,kind: LayoutKind, pos: Demision) {
        assert!(self.layouts.is_empty());
        self.layouts.push(Layout{
            kind,
            pos,
            size: Demision::new(0,0) 
        })
    }

    fn begin_layout(&mut self, kind: LayoutKind){ 
        let layout = self.layouts.last().expect("Can't create a layout outside of Ui::begin() and Ui::end()");
        let pos = layout.get_pos();

        self.layouts.push(Layout{
            kind,
            pos,
            size: Demision::new(0,0)
        })
    }

    // fn begin_list(&mut self, id: Id) {
    //     assert!(self.list_curr.is_none(), "Nested list are not allowed.");
    //     self.list_curr = Some(id);
    // }

    fn label(&mut self, text: &str, pair: i16){
        let layout = self.layouts.last_mut().expect("Trying to render label outside of any layouts.");
        let pos = layout.get_pos();
        mv(pos.y, pos.x);

        attron(COLOR_PAIR(pair));
        addstr(text).unwrap();
        attroff(COLOR_PAIR(pair));

        layout.add_widget(Demision::new(text.len() as i32, 1));
    }

    // fn list_element(&mut self, text: &str, id: Id) {
    //     let id_curr = self.list_curr.expect("Not allowed to create list element outside of lists");
    //     self.label(text, if id_curr == id {
    //         HIGHLIGHT_PAIR   
    //     } else {
    //         REGULAR_PAIR
    //     });
    // }

    // fn end_list(&mut self) {
    //     self.list_curr = None;
    // }

    fn end_layout(&mut self){
        let layout = self.layouts.pop().expect("Unbalanced Ui::begin_layout() and Ui::end_layout() calls.");
        self.layouts.last_mut()
            .expect("Unbalanced Ui::begin_layout() and Ui::end_layout() calls.")
            .add_widget(layout.size);
    }
    
    fn end(&mut self) {
        self.layouts.pop().expect("Unbalanced Ui::begin() and Ui::end() calls.");
    }
}

#[derive(PartialEq, PartialOrd)]
enum Status {
    Todo,
    Done
}

impl Status {
    fn toggle(self) ->  Self {
        match self {
            Status::Done => Status::Todo,
            Status::Todo => Status::Done
        }
    }

    fn right(self) -> Self {
        Status::Done
    }

    fn left(self) -> Self {
        Status::Todo
    }
}

fn list_up(list_curr: &mut usize) {
    if *list_curr > 0 { 
        *list_curr -= 1; 
    }
}

fn line_parse(line: &str) -> Option<(Status, &str)> {
    const TODO_PREFIX: &str = "TODO: ";
    const DONE_PREFIX: &str = "DONE: ";

    if line.starts_with(TODO_PREFIX) {
        return Some((Status::Todo, &line[TODO_PREFIX.len()..]))
    }

    if line.starts_with(DONE_PREFIX) {
        return Some((Status::Done, &line[DONE_PREFIX.len()..]))
    }

    return None;
}

fn list_down(list: &Vec<String>, list_curr: &mut usize) {
    if *list_curr + 1 < list.len() { 
        *list_curr += 1; 
    }
}

fn list_transfer(list_src: &mut Vec<String>, list_trans: &mut Vec<String>, list_curr: &mut usize) {
    if *list_curr < list_src.len() {
        list_trans.push(list_src.remove(*list_curr));
        if *list_curr >= list_src.len() && list_src.len() > 0 {
            *list_curr = list_src.len() - 1;
        }
    }
}

fn load_state(todos: &mut Vec<String>, dones: &mut Vec<String>, file_path: &str) {
    let file = File::open(file_path).unwrap();
    for (index,line) in BufReader::new(file).lines().enumerate() {
        if let Ok(i) = line {
            match line_parse(&i) {
                Some((Status::Todo, title)) => todos.push(title.to_string()),
                Some((Status::Done, title)) => dones.push(title.to_string()),
                None => {
                    eprintln!("{file_path}:{} Error: ill-formed item line", index + 1);
                    process::exit(1);
                }
            }
        }
    }
}

fn save_state(todos: &Vec<String>, dones: &Vec<String>, file_path: &str) {
    let mut file = File::create(file_path).unwrap();
    for todo in todos.iter() {
        writeln!(file, "TODO: {}", todo).unwrap();
    }
    for done in dones.iter() {
        writeln!(file, "DONE: {}", done).unwrap();
    }
}

fn main() {

    let mut args = env::args();
    args.next().unwrap();

    let file_path = match args.next() {
        Some(file_path) => file_path,
        None => {
            eprintln!("Usage yt_todo <file-path>");
            eprintln!("Error: file path is not provided");
            process::exit(1);
        }
    };

    let mut todos= Vec::<String>::new();
    let mut todo_curr: usize = 0;

    let mut dones = Vec::<String>::new();
    let mut done_curr: usize = 0;

    // loading the states
    load_state(&mut todos, &mut dones, &file_path);

    initscr();
    noecho(); // To not echo the keys typed

    // Allow for extended keyboard (like F1)
    keypad(stdscr(), true);

    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);  // make cursor invisible on terminal

    start_color();
    init_pair(REGULAR_PAIR, COLOR_WHITE, COLOR_BLACK);
    init_pair(HIGHLIGHT_PAIR, COLOR_BLACK, COLOR_WHITE);

    let mut quit = false;
    let mut tab = Status::Todo;

    let mut ui = Ui::default();

    const UP: char = KEY_UP as u8 as char;
    const DOWN: char = KEY_DOWN as u8 as char;
    const LEFT: char = KEY_LEFT as u8 as char;
    const RIGHT: char = KEY_RIGHT as u8 as char;

    while !quit {
        erase(); // to clean terminal before printing anything.
        ui.begin(LayoutKind::Horz,Demision::new(0, 0));
        {
            ui.begin_layout(LayoutKind::Vert);
            {
                ui.label("TODO:", REGULAR_PAIR);
                // ui.label("------------", REGULAR_PAIR);
                // ui.begin_list(todo_curr);
                for (index, todo) in todos.iter().enumerate() {
                    // ui.list_element(&format!("- [ ] {}",todo), index);
                    ui.label(&format!("- [ ] {todo}"), 
                        if index == todo_curr && tab == Status::Todo {
                            HIGHLIGHT_PAIR
                        } else {
                            REGULAR_PAIR
                        }
                    )
                }
            }
            ui.end_layout();

            ui.begin_layout(LayoutKind::Vert);
            {
                ui.label("DONE:", REGULAR_PAIR);
                // ui.label("------------", REGULAR_PAIR);
                // ui.begin_list(done_curr);
                for (index, done) in dones.iter().enumerate() {
                    // ui.list_element(&format!("- [x] {}",done),index);
                    ui.label(&format!("- [*] {done}"), 
                        if index == done_curr && tab == Status::Done {
                            HIGHLIGHT_PAIR
                        } else {
                            REGULAR_PAIR
                        }
                    )
                }
            }
            ui.end_layout();
            // ui.end_list();
        }
        ui.end();

        refresh();

        let key = getch();
        // mv(20,0);
        match key as u8 as char {
            'q' => quit = true,
            'e' => {
                let mut file = File::create("TODO").unwrap();
                for todo in todos.iter() {
                    writeln!(file, "TODO: {}", todo).unwrap();
                }
                for done in dones.iter() {
                    writeln!(file, "DONE: {}", done).unwrap();
                }
            },
            'w' | UP => {
                match tab {
                    Status::Todo => list_up(&mut todo_curr),
                    Status::Done => list_up(&mut done_curr)
                }
            },
            's' | DOWN => {
                match tab {
                    Status::Todo => list_down(&todos, &mut todo_curr),
                    Status::Done => list_down(&dones, &mut done_curr)
                }
            },
            '\n' => {
                match tab {
                    Status::Todo => list_transfer(&mut todos, &mut dones, &mut todo_curr),
                    Status::Done => list_transfer(&mut dones, &mut todos, &mut done_curr)
                }
            },
            'i' => {
                todo!();
            },
            '\t' => {
                tab = tab.toggle();
            },
            RIGHT => {
                tab = tab.right();
            },
            LEFT => {
                tab = tab.left();
            }
            _ => {}
        }
    }

    save_state(&todos, &dones, &file_path);

    endwin();
}
