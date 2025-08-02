use iced::{
    Element,
    alignment::Vertical,
    widget::{
        button, column, container, horizontal_rule, horizontal_space, row, text, vertical_space,
    },
};

pub struct Scaffold<'a, Message> {
    title: Option<Element<'a, Message>>,
    controls: Vec<Element<'a, Message>>,
    on_next: Option<Message>,
    on_back: Option<Message>,
    is_finished: bool,
}

impl<'a, Message> Scaffold<'a, Message> {
    pub fn new() -> Self {
        Self {
            title: None,
            controls: Vec::new(),
            on_next: None,
            on_back: None,
            is_finished: false,
        }
    }

    pub fn title(mut self, title: impl Into<Element<'a, Message>>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn control(mut self, control: impl Into<Element<'a, Message>>) -> Self {
        self.controls.push(control.into());
        self
    }

    pub fn on_next(mut self, message: Message) -> Self {
        self.is_finished = false;
        self.on_next = Some(message);
        self
    }

    pub fn on_next_maybe(mut self, message: Option<Message>) -> Self {
        self.is_finished = false;
        self.on_next = message;
        self
    }

    pub fn on_back(mut self, message: Message) -> Self {
        self.on_back = Some(message);
        self
    }

    pub fn on_back_maybe(mut self, message: Option<Message>) -> Self {
        self.on_back = message;
        self
    }

    pub fn on_finish(mut self, message: Message) -> Self {
        self.is_finished = true;
        self.on_next = Some(message);
        self
    }

    pub fn on_finish_maybe(mut self, message: Option<Message>) -> Self {
        self.is_finished = true;
        self.on_next = message;
        self
    }
}

impl<'a, Message> From<Scaffold<'a, Message>> for Element<'a, Message>
where
    Message: 'a + Clone,
{
    fn from(scaffold: Scaffold<'a, Message>) -> Self {
        let rule = scaffold.title.as_ref().map(|_| horizontal_rule(2));
        column![]
            .push_maybe(
                scaffold
                    .title
                    .map(|title| container(title).height(30).align_y(Vertical::Bottom)),
            )
            .push_maybe(rule)
            .push(column(scaffold.controls.into_iter()).spacing(20))
            .push(vertical_space())
            .push(horizontal_rule(2))
            .push(
                row![
                    horizontal_space(),
                    button(text("Back"))
                        .padding([8, 30])
                        .on_press_maybe(scaffold.on_back),
                    button(text(if scaffold.is_finished {
                        "Finish"
                    } else {
                        "Next"
                    }))
                    .padding([8, 30])
                    .on_press_maybe(scaffold.on_next)
                ]
                .spacing(20)
                .padding(10),
            )
            .spacing(10)
            .padding(20)
            .into()
    }
}
