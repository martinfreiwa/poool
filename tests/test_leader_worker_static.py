from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_returned_leader_worker_backs_off_before_reacquiring():
    leader = (ROOT / "backend/src/common/leader.rs").read_text()
    returned_block = leader.split("worker().await;", 1)[1]

    assert "worker returned — leader lock released, retrying in {}s" in returned_block
    assert "tokio::time::sleep(Duration::from_secs(RETRY_SECS)).await;" in returned_block
