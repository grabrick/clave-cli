pub(crate) fn previous_boundary(input: &str, cursor: usize) -> usize {
    input[..cursor]
        .char_indices()
        .last()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

pub(crate) fn next_boundary(input: &str, cursor: usize) -> usize {
    input[cursor..]
        .char_indices()
        .nth(1)
        .map(|(index, _)| cursor + index)
        .unwrap_or_else(|| input.len())
}

pub(crate) fn previous_word_boundary(input: &str, cursor: usize) -> usize {
    if cursor == 0 {
        return 0;
    }

    let mut position = cursor;
    while position > 0 {
        let previous = previous_boundary(input, position);
        let ch = input[previous..position].chars().next().unwrap_or(' ');
        if !ch.is_whitespace() {
            break;
        }
        position = previous;
    }

    if position == 0 {
        return 0;
    }

    let previous = previous_boundary(input, position);
    let word_mode = is_word_char(input[previous..position].chars().next().unwrap_or(' '));

    while position > 0 {
        let previous = previous_boundary(input, position);
        let ch = input[previous..position].chars().next().unwrap_or(' ');
        if ch.is_whitespace() || is_word_char(ch) != word_mode {
            break;
        }
        position = previous;
    }

    position
}

pub(crate) fn next_word_boundary(input: &str, cursor: usize) -> usize {
    if cursor >= input.len() {
        return input.len();
    }

    let mut position = cursor;
    while position < input.len() {
        let next = next_boundary(input, position);
        let ch = input[position..next].chars().next().unwrap_or(' ');
        if !ch.is_whitespace() {
            break;
        }
        position = next;
    }

    if position >= input.len() {
        return input.len();
    }

    let next = next_boundary(input, position);
    let word_mode = is_word_char(input[position..next].chars().next().unwrap_or(' '));

    while position < input.len() {
        let next = next_boundary(input, position);
        let ch = input[position..next].chars().next().unwrap_or(' ');
        if ch.is_whitespace() || is_word_char(ch) != word_mode {
            break;
        }
        position = next;
    }

    position
}

pub(crate) fn line_start_boundary(input: &str, cursor: usize) -> usize {
    input[..cursor]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0)
}

pub(crate) fn line_end_boundary(input: &str, cursor: usize) -> usize {
    input[cursor..]
        .find('\n')
        .map(|index| cursor + index)
        .unwrap_or_else(|| input.len())
}

pub(crate) fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}
