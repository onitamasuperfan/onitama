import React, { useCallback, useState } from 'react';
import { useSnackbar } from 'notistack';
import useLocalGame from './hooks/useLocalGame';
import Loading from './Loading';
import GameBoard from './GameBoard';
import getMoves from './utils/moveUtils';

function LocalGame() {
  const { enqueueSnackbar } = useSnackbar();
  const { state, playMove, reset } = useLocalGame();
  const [card, setCard] = useState(null);
  const [src, setSrc] = useState(null);

  const move = useCallback(
    ({ x, y, revealNinja }) => {
      if (!card || !src) {
        return;
      }
      if (!playMove) {
        enqueueSnackbar('Game loading, try again', { variant: 'warning' });
        return;
      }
      const action = {
        card: card.card,
        src,
        dst: { x, y },
        reveal_ninja: revealNinja,
        type: 'Move',
      };

      const { grid } = state; // Destructure `grid` from `state`
      const previousTile = grid[src.y]?.[src.x]; // Tile being moved from
      const destinationTile = grid[y]?.[x]; // Tile being moved to

      const isHiddenNinja = (tile) =>
        tile &&
        typeof tile === 'object' &&
        Object.keys(tile)[0].includes('Ninja') &&
        !tile[Object.keys(tile)[0]].revealed;

      const error = playMove(action);
      if (error) {
        enqueueSnackbar(error, { variant: 'error' });
      } else {
        setCard(null);
        setSrc(null);
      }

      // Check if a hidden Ninja was captured (and exclude interactions between hidden Ninjas)
      if (
        isHiddenNinja(destinationTile) && // The tile being moved to contains a hidden Ninja
        !isHiddenNinja(previousTile) // The piece being moved is not also a hidden Ninja
      ) {
        enqueueSnackbar('You captured their hidden Ninja!', { variant: 'success' });
      }
    },
    [playMove, src, card, enqueueSnackbar],
  );

  const discard = useCallback(
    (discardCard) => {
      if (!playMove) {
        enqueueSnackbar('Game loading, try again', { variant: 'warning' });
        return;
      }
      const action = { card: discardCard, type: 'Discard' };
      const error = playMove(action);
      if (error) {
        enqueueSnackbar(error, { variant: 'error' });
      } else {
        setCard(null);
        setSrc(null);
      }
    },
    [playMove, enqueueSnackbar],
  );

  if (!state) {
    return <Loading />;
  }

  const {
    blueCards,
    redCards,
    spare,
    turn,
    grid,
    canMove,
    winner,
    windMovePending,
    windMoveCard,
    ninjaMovePending,
    ninjaMoveCard,
  } = state;

  const isMoveValid = getMoves(
    src,
    card,
    grid,
    turn,
    windMovePending,
    ninjaMovePending,
    ninjaMoveCard,
  );

  return (
    <GameBoard
      src={src}
      setSrc={setSrc}
      card={card}
      setCard={setCard}
      blueCards={blueCards}
      redCards={redCards}
      grid={grid}
      isMoveValid={isMoveValid}
      canMove={canMove}
      reset={reset}
      winner={winner}
      spare={spare}
      turn={turn}
      move={move}
      discard={discard}
      windMovePending={windMovePending}
      windMoveCard={windMoveCard}
      ninjaMovePending={ninjaMovePending}
      ninjaMoveCard={ninjaMoveCard}
    />
  );
}

export default LocalGame;
