export const Example1 = `

shared {
  let Done = false;
  let Leader = 0;
}

program A(my_id: int) {

  let leader_id = my_id;

  send out(my_id);

  loop atomic wait receive in (x) => {
    print("receive", x);
      if x > leader_id {
        leader_id = x;
        send out(x);
      } else {
        if x == leader_id {
          print("finished");
          send out(x);
          break;
        }
      }
  };
  
  if my_id == leader_id {
    print("I AM THE LEADER!!!");
    ! {
        Done = true;
        Leader += 1;
    }
  }
}

always {
    !Done || (Leader == 1);
}

main {
  let n = 4;
  let a:list(proc(A));
  for i in 0..n {
    let p = run A(i);
    a.push(p);
  }
  for i in 0..n-1 {
    let p1 = a.at(i);
    let p2 = a.at(i+1);
    channel p1.out (int)> p2.in;
  }
  
  let p1 = a.at(n-1);
  let p2 = a.at(0);
  channel p1.out (int)> p2.in;

  print("DONE");
}

`;
