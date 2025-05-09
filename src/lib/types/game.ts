export type GameDifficulty = "easy" | "medium" | "hard" | "expert" | "custom";
export type GameType = "directory" | "file";

export interface Game {
	id: number;
	name: string;
	difficulty: GameDifficulty;
	level: number;
	timeLimit: number; // in seconds
	hintCount: number;
	totalLevels: number;
	gameData?: GameData;
}

export interface Level {
	answer: string;
}

export interface GameData {
	correctAnswers: number;
	closeAnswers: number;
	incorrectAnswers: number;
	startTime: Date | null;
	endTime: Date | null;
    levels: Level[];
}

export interface GameSettings {
	type: GameType;
	difficulty: Exclude<GameDifficulty, "custom">;
	hintCount: number;
	timeLimit: number; // in seconds
	totalLevels: number;
	closeSensitivity: number; // percentage?
}
