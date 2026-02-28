from dataclasses import dataclass
from typing import Callable


type Cell = list[int]
type Block = int


@dataclass
class VLine:
    block: int
    value: int


@dataclass
class Board:
    cells: list[Cell]
    row_vlines: list[VLine]
    col_vlines: list[VLine]


def make_cell(value: int) -> Cell:
    UNFILLED = [1, 2, 3, 4, 5, 6, 7, 8, 9]
    return UNFILLED if value == 0 else [value]


def make_board(board: list[int]) -> Board:
    assert len(board) == 81
    return Board([make_cell(cell) for cell in board], [], [])


def eliminate_by(board: Board, indexes_fn: Callable[[int], list[int]]) -> Board:
    for i in range(9):
        indexes = indexes_fn(i)

        eliminate_values = [
            cell[0] for cell in (board.cells[i] for i in indexes) if len(cell) == 1
        ]
        for index in indexes:
            for v in eliminate_values:
                cell = board.cells[index]
                if v in cell and len(cell) != 1:
                    board.cells[index].remove(v)

    return board


def eliminate(board: Board) -> Board:
    board = eliminate_by(
        board,
        lambda row: [row * 9 + col for col in range(9)],
    )
    board = eliminate_by(
        board,
        lambda col: [row * 9 + col for row in range(9)],
    )
    board = eliminate_by(
        board,
        lambda blk: [
            row * 9 + col
            for row in range((blk // 3) * 3, (blk // 3) * 3 + 3)
            for col in range((blk % 3) * 3, (blk % 3) * 3 + 3)
        ],
    )
    return board


def detect_vline(board: Board) -> Board:
    for blk in range(9):
        blk_row = blk // 3
        blk_col = blk % 3

        row = blk_row * 3
        for col in range(blk_col * 3, blk_col * 3 + 3):
            index = row * 9 + col
            cell = board.cells


def print_board(board: Board) -> None:
    for row in range(9):
        s = ""
        for col in range(9):
            cell = board.cells[row * 9 + col]
            if len(cell) == 1:
                s += "\x1b[1;32m"
                s += f" {cell[0]} "
                s += "\x1b[m"
            else:
                s += "\x1b[2;34m"
                s += f" {len(cell)} "
                s += "\x1b[m"
        print(s)


board = make_board(
    [
        0,
        0,
        5,
        0,
        0,
        8,
        6,
        7,
        0,
        0,
        6,
        9,
        0,
        5,
        0,
        1,
        2,
        0,
        0,
        0,
        1,
        4,
        0,
        0,
        9,
        5,
        0,
        0,
        0,
        2,
        6,
        0,
        0,
        5,
        4,
        7,
        6,
        0,
        4,
        0,
        7,
        5,
        2,
        3,
        1,
        5,
        0,
        7,
        0,
        0,
        4,
        8,
        9,
        6,
        1,
        4,
        6,
        5,
        0,
        0,
        7,
        8,
        9,
        9,
        5,
        8,
        7,
        4,
        1,
        3,
        6,
        2,
        0,
        0,
        3,
        0,
        0,
        0,
        4,
        1,
        5,
    ]
)
for i in range(30):
    board = eliminate(board)
print_board(board)
