export interface GameStatistics {
    total: number; // total number of levels played
    win: number; // a "win" is more than 50% correct answers i guess? can define percentage in settings ig
    lose: number; // a "lose" is less than 50% correct answers
    close: number; // total of "close" answers in all levels (level[])
    hintsUsed: number; // total of all hints used in all levels (level[])
    timeTaken: number; // total of all time taken in all levels (level[])
}

export interface LevelStatistics {
    guesses: string[]; // all guesses made in the level
    guessesCount: number;
    closeGuesses: number;
    hintsUsed: number;
    timeTaken: number;
}