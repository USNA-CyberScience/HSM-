/* 
 * 
 *
 * 
 * Will Casey
 */
use argh::FromArgs;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, size},
};
use std::{error::Error, io, fmt};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Spans, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState, List, ListItem, ListState, Paragraph, Wrap, Clear},
    Frame, Terminal,
};

use unicode_width::UnicodeWidthStr;

enum InputMode {
    Normal,
    Editing,
}


//use std::error::Error;
use csv;
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use serde::Deserialize;

use crate::mitre12::*;
pub mod mitre12; 
use mitre12::Hnode;


/// Demo
#[derive(Debug, FromArgs)]
struct cmd_args {
    #[argh(positional, description="input file (csv)")]
    input: String,
    #[argh(option, short = 'o',  description="output file path")]
    output: Option<String>,
}

/*
#[derive(Debug, Deserialize)]
struct data_row {
    time: String,
    keyraw: String,
    keyparse: String,
    cmdname: String,
    error: f32,
//    mitre: String,
    //comment: String
}
 */

/// Reads keylog data from a file into a reader and deserializes each record
///
/// # Error
///
/// If an error occurs, the error is returned to `main`.
fn read_keylog_from_file(path: &str) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let mut rdr = ReaderBuilder::new()
	.delimiter(b'\t')
	.has_headers(false)
	.from_path(path)?;

    let mut rv : Vec<Vec<String>> = vec![];
    for result in rdr.records() {
        let record  = result?;
	let vx : Vec<String> =  record.iter().map(|s| String::from( s ) ).collect();
        //println!("{:?}", vx);
	rv.push( vx );
    }

    Ok(rv)
	
}

/// Inserts data into writer and writes to a file
///
/// # Error
///
/// If an error occurs, the error is returned to `main`
fn write_to_file(path: &str, DATA : & Vec<Vec<String>>) -> Result<(), Box<dyn Error>> {

    let mut wtr = WriterBuilder::new()
        .delimiter(b'\t')
        .from_path(path)?;

    for D in DATA{
	wtr.write_record(*&D)?;
    }

    wtr.flush()?;

    Ok(())
}


struct App<'a> {
    //state  : Vec<Option<&'a Hnode<'a>>>,
    istate : (i32, i32, i32), 
    ostate : Vec<Option<Vec<&'a str>>>,
    // point to the next.
    lstate : Vec<ListState>,
    cur    : u8,
    x      : Hnode<'a>,
    tabstate: TableState,
    tabdata : &'a mut Vec<Vec<String>>,
    modal_state : u8,
    input_mode: InputMode,
    input     : String,
    output    : String, 
}


impl<'a> App<'a> {
    pub fn new( d : &'a mut  Vec<Vec<String>> ) -> App<'a> {
        App {
	    //state  : vec![ None, None, None ], // these are the Hnodes ( positions in the MITRE12 tree )
	    ostate : vec![ None, None, None ], // option states ( is the listing of strings located in each Hnode. ) if hnode then vec exists
	    istate : (-1,-1,-1), // these represent the position of the cursor iterating over the ostate 0 by default
	    lstate : vec![ListState::default(),ListState::default(),ListState::default()], // manages the interface
  	    cur    : 0, 
	    x      : mitre12::mitre12(),
	    // for the table
	    tabstate: TableState::default(),
	    tabdata : d,
	    // for the modal windows
	    modal_state : 0,
	    // for the comment field 
	    input_mode : InputMode::Normal,
	    input : String::new(),
	    output: String::from("output.csv"),
        }
    }

    pub fn set_output( & mut self, output : String ){
	self.output = output; 
    }
    pub fn get_info( & self ) -> Vec<String> {
	let mut rv = Vec::<String>::new() ;
	if let Some( QX ) = & self.ostate[0]{
	    let Q = QX[self.istate.0 as usize ];
	    let V = self.x.get_des( Q ).replace( "safe_str(", "Categroy: " );
	    let V = V[0..(V.len()-1)].to_string();
	    rv.push( V) //;
	};
	if let Some( QX ) = &self.ostate[1]{
	    let Q = QX[self.istate.1 as usize ];
	    if QX.len() > 0 {
		if let Some(VX) = self.get_hnode( vec![self.istate.0]){
		    let V = VX.get_des( Q ).replace( "safe_str", "tech: " );
		    rv.push( String::from("technique: ")  + &V );
		}
	    }
	}
	if let Some(QX ) = &self.ostate[2]{
	    if QX.len() > 0 {
		let Q = QX[self.istate.2 as usize ];  
		if let Some(VX) = self.get_hnode( vec![self.istate.0, self.istate.1]){
		    let V = VX.get_des( Q ).replace( "safe_str", "" );
		    rv.push( String::from( "more: ") + &V );
		}
	    }
	}
	rv 
    }
    pub fn get_hnode(& self, it: Vec<i32> ) -> Option< &Hnode<'a>>{
	let mut cur = &self.x	;
	for k in it {
	    let VM = cur.get_listing().to_vec() ;
	    if ( k < VM.len() as i32) && ( k > -1 ){
		let QS = VM[k as usize ];
		let next = cur.get_child( QS );
		if let Some( ni ) = next{
		    cur = ni
		}
		else {
		    return None
		}
	    } else {
		return None
	    }
	}
	Some( cur ) 
    }

    pub fn update_index( &mut self, inde : (i32, i32, i32 )){
	/* set first menu */

	self.ostate[0] = Some( self.x.get_listing().to_vec() ); // safe if self.x
	self.istate.0 = inde.0;	
	self.lstate[0].select(Some(self.istate.0 as usize));

	/* set second menu */
	if self.istate.0 > -1 {
	    if self.ostate[0].as_ref().expect( "ostate error l0" ).len() > 0  {
		let y = self.get_hnode( vec![self.istate.0] );
		if let Some( Y ) = y {
		    self.ostate[1] = Some( Y.get_listing().to_vec() ); 
		    self.istate.1 = inde.1 ;
		    self.lstate[1].select(Some( self.istate.1 as usize));
		    /* thrid menu */
		    let y = self.get_hnode( vec![self.istate.0, self.istate.1]);
		    if let Some(Y ) = y {
			self.ostate[2] = Some( Y.get_listing().to_vec() );
			self.istate.2 = inde.2 ;
			self.lstate[2].select( Some(  self.istate.2 as usize));
		    } else {
			self.ostate[2] = None;
		    }
		} else {
		    self.ostate[1] = None;
		}
	    }
	}
    }
    pub fn init( &mut self ) {
	self.update_index( (0,0,0));
    }

    pub fn tab_next(&mut self) {
        let i = match self.tabstate.selected() {
            Some(i) => {
                if i >= self.tabdata.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.tabstate.select(Some(i));
    }
    pub fn tab_previous(&mut self) {
        let i = match self.tabstate.selected() {
            Some(i) => {
                if i == 0 {
                    self.tabdata.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.tabstate.select(Some(i));
    }


    pub fn tag_row( &mut self){
	if let Some( i ) = self.tabstate.selected() {
	    self.tabdata[ i ][ 5] = String::from( format!("{:?}", self.istate));
	}
    }

    pub fn comment_row( &mut self, com : String){
	if let Some( i ) = self.tabstate.selected() {
	    self.tabdata[ i ][ 6] = com.clone();
	}
    }

    
    pub fn modal( &mut self, sx : u8){
	self.modal_state = self.modal_state ^ sx;
    }
    
    /*      */
    pub fn focus_move(&mut self, unit : i32 ) {
	match self.cur {	   
	    0 => {
		if let Some( Q ) = &self.ostate[0] {
		    let qlen = Q.len();
		    let ilen =  if qlen > 0 { qlen } else { 1 };				
		    let i = match self.lstate[0].selected() {
			Some(i) => (((ilen as i32) + (i as i32) + unit ) % (ilen as i32 )) as usize,
			None => 0};
		    self.update_index( ( i as i32, 0, 0 ));
		}
	    },
	    1 => {
		if let Some( Q ) = &self.ostate[1] {
		    let qlen = Q.len();
		    let ilen =  if qlen > 0 { qlen } else { 1 };		
		    let i = match self.lstate[1].selected() {
			Some(i) => (((ilen as i32) + (i as i32) + unit ) % (ilen as i32 )) as usize,
			None => 0};
		    self.update_index( ( self.istate.0, i as i32, 0 ));
		}
	    },
	    2 => {
		if let Some( Q ) = &self.ostate[2] {
		    let qlen = Q.len();
		    let ilen =  if qlen > 0 { qlen } else { 1 };
		    let i = match self.lstate[2].selected() {
			Some(i) => (((ilen as i32) + (i as i32) + unit ) % (ilen as i32 )) as usize,
			None => 0};
		    self.update_index( ( self.istate.0, self.istate.1, i as i32 ));
		}
	    }
	    _ => {}
	}
	
    }
    pub fn next( &mut self ){
	self.focus_move(  1 );
    }
    pub fn previous(&mut self) {
	self.focus_move( -1 );
    }

    pub fn tabover(&mut self ){
	self.cur = (self.cur  + 1)%3;
	
    }

}

fn main() -> Result<(), Box<dyn Error>> {
    let cmd : cmd_args = argh::from_env();
    //println!( " Cli = {:?}", cmd  );
    
    let mut data = read_keylog_from_file(&cmd.input)?;

    let output = match cmd.output {
	Some( x ) => x,
	None => String::from( cmd.input.clone() ),
    };
    /*     */
    
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::new( & mut data );
    app.init();    
    app.set_output( output.clone() ) ;
    
    let res = run_app(&mut terminal, app);
    
    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }
    //println!( " here {:?}", app  ) ; 
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
	    match app.input_mode {
		InputMode::Normal => match key.code {
                    KeyCode::Char('q') => return write_to_file( &app.output , app.tabdata ), 
                    KeyCode::Down => if app.modal_state == 0 { app.next() } else {},
                    KeyCode::Up => if app.modal_state == 0 { app.previous() } else {},
                    KeyCode::Tab => if app.modal_state == 0 { app.tabover() } else {},
                    KeyCode::Char('d') => if app.modal_state == 0 { app.tab_next() } else {}, 
                    KeyCode::Char('e') => if app.modal_state == 0 { app.tab_previous() } else {},
                    KeyCode::Char('h') => if (app.modal_state == 0 )||( app.modal_state == 1 ) { app.modal( 1 ) } else {},
                    KeyCode::Char('v') => if (app.modal_state == 0 )||( app.modal_state == 2 ) { app.modal( 2 ) } else {},
                    KeyCode::Char('c') => if (app.modal_state == 0 ){
			if let Some( i ) = app.tabstate.selected() {
			    app.modal( 4 ); app.input_mode = InputMode::Editing;
			}
		    } else {},
                    KeyCode::Enter => if (app.modal_state == 0 ) { app.tag_row() } else {},
                    _ => {}
		},
		InputMode::Editing => match key.code {
                    KeyCode::Enter => {
                        let COMMENT: String = app.input.drain(..).collect();
			app.comment_row( COMMENT );
                        app.input_mode = InputMode::Normal;
			app.modal( 4 );		
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Esc => {
                        let COMMENT: String = app.input.drain(..).collect();			
                        app.input_mode = InputMode::Normal;
			app.modal( 4 );					
                    }
                    _ => {}
		}
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(20),
                Constraint::Length(20),
                Constraint::Length(10),		
            ]
            .as_ref(),
        )
        .split(f.size());

    ///////////////////////////////////////////////////////////////
    // table area  

    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let normal_style = Style::default().bg(Color::LightBlue);
    let header_cells = ["time", "keycap", "shell", "cmd name", "key acc", "TAG", "comment" ]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Red)));
    let header = Row::new(header_cells)
        .style(normal_style)
        .height(1)
        .bottom_margin(1);
    let rows = app.tabdata.iter().map(|item| {
        let height = item
            .iter()
            .map(|content| content.chars().filter(|c| *c == '\n').count())
            .max()
            .unwrap_or(0)
            + 1;
        let cells = item.iter().map(|c| Cell::from(c.clone()));
        Row::new(cells).height(height as u16).bottom_margin(0)
    });
    let t = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Table"))
        .highlight_style(selected_style)
        .highlight_symbol(">> ")
        .widths(&[
            Constraint::Percentage(10),//time 
            Constraint::Percentage(10),//keycap
            Constraint::Percentage(20),//shell
            Constraint::Percentage(10),//cmd name
            Constraint::Percentage(10),//key acc
            Constraint::Percentage(10),
            Constraint::Percentage(5),			    
        ]);

    f.render_stateful_widget(t , chunks[0], &mut app.tabstate); 
    /*   */

    ///////////////////////////////////////////////////////////////
    // mitre menu selection area 

    let menu_area = Layout::default()
	.direction(Direction::Horizontal)
	.constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)].as_ref())
	.split( chunks[1]);

    
    let cats: Vec<ListItem> = match &( app.ostate[0] )
    {
	Some( x ) => x
	    .iter()
            .map(|i| ListItem::new(vec![Spans::from(Span::raw(*i))]))
            .collect(),
	None => { vec![ListItem::new(vec![Spans::from(Span::raw("none"))])] }
    };

    
    let cats = List::new(cats)
        .block(Block::default().borders(Borders::ALL).
	       title(
		   Span::styled(
		       "[ Frame work Category ]",
		       if  app.cur == 0 { 
			   Style::default()
			       .fg(Color::Green)
			       .add_modifier(Modifier::BOLD) } else { Style::default() },
		   )
	       ))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">");
    f.render_stateful_widget(cats, menu_area[0], &mut app.lstate[0]);

	
    let subcats: Vec<ListItem> = match &(app.ostate[1]) 
    {
	Some( x ) => x
	    .iter()
            .map(|i| ListItem::new(vec![Spans::from(Span::raw(*i))]))
            .collect(),
	None => { vec![ListItem::new(vec![Spans::from(Span::raw("none"))])] }
    };
    

    let subcats = List::new(subcats)
        .block(Block::default().borders(Borders::ALL).title(
		   Span::styled(
		       "[ techniques ]",
		       if  app.cur == 1 { 
			   Style::default()
			       .fg(Color::Green)
			       .add_modifier(Modifier::BOLD) } else { Style::default() },
		   )
	))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">");
    f.render_stateful_widget(subcats, menu_area[1], &mut app.lstate[1]);


    let subsubcats: Vec<ListItem> = match &(app.ostate[2]) 
    {
	Some( x ) => x
	    .iter()
            .map(|i| ListItem::new(vec![Spans::from(Span::raw(*i))]))
            .collect(),
	None => { vec![ListItem::new(vec![Spans::from(Span::raw("none"))])] }
    };
    
    let subsubcats = List::new(subsubcats)
        .block(Block::default().borders(Borders::ALL).title(
		   Span::styled(
		       "[ more ]",
		       if  app.cur == 2 { 
			   Style::default()
			       .fg(Color::Green)
			       .add_modifier(Modifier::BOLD) } else { Style::default() },
		   )
	))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">");
    f.render_stateful_widget(subsubcats, menu_area[2], &mut app.lstate[2]);
 
    
    let text1 : Vec<Spans> = app.get_info().iter().map( |i| Spans::from( i.clone() )).collect(); 
	
    let block1 = Block::default().borders(Borders::ALL).title(Span::styled(
        "[ info ]",
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    ));

    
    
    let paragraph1 = Paragraph::new(text1).block(block1).wrap(Wrap { trim: true });
    f.render_widget(paragraph1, chunks[2]);
    

    if app.modal_state == 1 {
        let block = Block::default().title(" [ help  ~~ (press h to return) ] ")
	    .borders(Borders::ALL)
	    .style(Style::default().bg(Color::Blue));

        let area = centered_rect(60, 34, f.size());

        f.render_widget(Clear, area); //this clears out the background

	let text : Vec<Spans> = vec![" navigation " , "table     framework navigation", "e up      <up arrow> up", "d down    <down arrow> down", ".         <tab> swtich menu", "    enter -> attribute and tag table line", "c  -> comment table line", "h -> help (toggle)", "q -> save and quite", "v -> view full table entry"].iter().map( |i| Spans::from( i.clone() )).collect(); 	
	let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });	
	f.render_widget(paragraph, area);	
    }


    if app.modal_state == 2 {
        let block = Block::default().title(" [ view data table row  ~~  (press v to return) ] ")
	    .borders(Borders::ALL)
	    .style(Style::default().bg(Color::Blue));

        let area = centered_rect(60, 34, f.size());

        f.render_widget(Clear, area); //this clears out the background

	if let Some(i) = app.tabstate.selected() {
	    let text : Vec<Spans> = app.tabdata[i].iter().map( |i| Spans::from( i.clone() )).collect();
	    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });	
	    f.render_widget(paragraph, area);
	}
	else {
	    let text : Vec<Spans> = ["move table cursor with e/d","h for help"].iter().map( |i| Spans::from( i.clone() )).collect();
	    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });	
	    f.render_widget(paragraph, area);

	}
	
    }

    if app.modal_state == 4 {
	
        let area = centered_rect(60, 34, f.size());
        f.render_widget(Clear, area); //this clears out the background
	
	let input = Paragraph::new(app.input.as_ref())
            .style(match app.input_mode {
		InputMode::Normal => Style::default(),
		InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::default().borders(Borders::ALL).title("[ comment table row  ~~ enter to commit / esc to abort  ]"));
	f.render_widget(input, area);
	match app.input_mode {
            InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}
	    
            InputMode::Editing => {
		// Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
		f.set_cursor(
                    // Put cursor past the end of the input text
                    area.x + app.input.width() as u16 + 1,
                    // Move one line down, from the border to the input line
                    area.y + 1,
		)
            }
	}
    

	
	
    }

    
    
    
}
 

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
