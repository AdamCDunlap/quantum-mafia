import React from 'react';

class AskForName extends React.Component {
  constructor() {
    super();
    this.state = {textVal: ''};
  }

  render() {
    return (
      <div>
        <input
          type="text"
          placeholder="Nickname"
          value={this.state.textVal}
          onChange={(evt) => this.setState({textVal: evt.target.value.substr(0, 20)})} />

        <button
          onClick={() => {
            this.props.ws_send_cmd({ SetName: this.state.textVal });
          }}>
          Set Nickname
        </button>
      </div>
    );
  }
}

class CreateGameButton extends React.Component {
  constructor() {
    super();
    this.state = {textVal: ''};
  }

  render() {
    return (
      <div>
        <input
          type="text"
          placeholder="Game Name"
          value={this.state.textVal}
          onChange={(evt) => this.setState({textVal: evt.target.value.substr(0, 20)})} />

        <button
          onClick={() => {
            this.props.ws_send_cmd({ CreateGame: this.state.textVal });
            this.setState({textVal: ''});
          }}>
          Create New Game
        </button>
      </div>
    );
  }
}

class WaitingRoom extends React.Component {
  render() {
    return (
      <div>
        <p> People in the waiting room: </p>
        <ul>{
          this.props.WaitingRoom.waiting_room_people.map((name) =>
              <li key={name}>{name}</li>
          )
        }</ul>
        <CreateGameButton ws_send_cmd={this.props.ws_send_cmd}/>
        <p> Games (Click to join): </p>
        <ul>{
          this.props.WaitingRoom.open_games.map(([id, name, players]) =>
            <li key={id}
              onClick={() => this.props.ws_send_cmd({
                  EnterGame: id
              })}>
                {players[0] + "'s game \"" + name + '"'}
              </li>
          )
        }</ul>
      </div>
    );
  }
}

function UnstartedGame(props) {
  return (
    <div>
      <p> You're in game {props.UnstartedGame.game_name} </p>
      <p> People in your game: </p>
      <ul>{
        props.UnstartedGame.people_in_game.map((name) =>
            <li key={name}>{name}</li>
        )
      }</ul>
    </div>
  );
}

class SetSubgameRoles extends React.Component {
  render() {
    return (
      <div>
        Roles
      </div>
    );
  }
}

class App extends React.Component {

  constructor() {
    super();

    let ws = new WebSocket("ws://127.0.0.1:3918");

    ws.onmessage = function(app) {
      return function(e) {
        console.log(e.data);

        app.setState({
          ws: app.state.ws,
          state: JSON.parse(e.data),
        });
        //let msg = JSON.parse(e.data);

        //if (msg.WaitingRoomUpdate) {
        //  let newstate = {};
        //  for (let x in app.state) newstate[x] = app.state[x];
        //  newstate.waiting_room = msg.WaitingRoomUpdate;
        //  app.setState(newstate);
        //} else if (msg.GameUpdate) {
        //  let newstate = {};
        //  for (let x in app.state) newstate[x] = app.state[x];
        //  newstate.waiting_room_people = msg.WaitingRoomUpdate;
        //  app.setState(newstate);
        //}

        ////if (app.state.state === 'confirm_nickname') {
        ////  let newstate = {};
        ////  for (let x in app.state) newstate[x] = app.state[x];
        ////  newstate.state = 'waiting_room';
        ////  app.setState(newstate);
        ////} else if (app.state.state === 'waiting_room') {

        ////} else {
        ////  console.log("Got message at unexpected time");
        ////}
      }
    }(this);

    this.state = {
      ws: ws,
      state: {SetNickname:{}},
    };
  }

  render() {
    let ws_send_cmd = function(app) { return function(cmd) {
      app.state.ws.send(JSON.stringify({cmd: cmd}));
    }}(this);
    if (this.state.state.SetNickname) {
      return (<AskForName
                ws_send_cmd={ws_send_cmd} />);
    } else if (this.state.state.WaitingRoom) {
      console.log(this.state);
      return ( <WaitingRoom
                ws_send_cmd={ws_send_cmd}
                WaitingRoom={this.state.state.WaitingRoom} />);
    } else if (this.state.state.UnstartedGame) {
      return ( <UnstartedGame
                ws_send_cmd={ws_send_cmd}
                UnstartedGame={this.state.state.UnstartedGame} />);
    } else if (this.state.state.SetSubgameroles) {
      return <SetSubgameRoles />;
    } else {
      console.log("Unexpected state");
      console.log(this.state);
    }
  }
}

export default App;
